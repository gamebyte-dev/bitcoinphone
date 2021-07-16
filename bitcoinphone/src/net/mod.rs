use std::collections::HashMap;
use std::ops::{AddAssign, Sub};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{Receiver, Sender, sync_channel, SyncSender};
use std::thread;
use std::time::Duration;

use sv::messages::{FilterAdd, Inv, INV_VECT_TX, InvVect, Message, Tx};
use sv::peer::{Peer, PeerDisconnected};
use sv::util::Hash160;
use sv::util::rx::Observer;

use lazy_static::lazy_static;
use peer_db::{PeerDB, SafePeerDB};
use peerman::{PeerMan, PPeer};
use peerman::filter::BloomFilterState;
use peerman::message_handler::MessageHandler;

use crate::net::addr_bus::{AddrBus, AddressAction, IpTuple, AddrEvent};
use crate::net::addr_bus::AddrOp;
pub use crate::net::tx_bus::{TxBus, TxEvent, TxOperation};
use sv::script::Script;
use std::cmp::min;
use crate::util::constants::{UTXOPacket, DataPacket, Key};
use crate::util::traits::Spawnable;
use sv::transaction::p2pkh::create_lock_script;

mod peer_db;
mod tx_bus;

mod peerman;
mod addr_bus;

pub const MAX_FILTER_ITEMS: f64 = 10.0;
pub const MAX_CONCURRENT_HANDSHAKES: usize = 100;
pub const MAXIMUM_HANDSHAKE_ATTEMPTS: usize = 20;

pub struct NetworkInterface {
    peerman: Arc<PeerMan>,
    peer_db: SafePeerDB,
    addr_bus_sender: SyncSender<AddrEvent>,
    tx_bus_sender: SyncSender<TxEvent>,
    filter: Arc<BloomFilterState>,
    handler: Arc<MessageHandler>,
}

impl NetworkInterface {
    pub fn new(
        payment_sender: SyncSender<UTXOPacket>,
        data_sender: SyncSender<DataPacket>
    ) -> Self {
        let peer_db = PeerDB::new();
        let peer_db_sender = peer_db.spawn_gateway();
        let addr_bus_sender = AddrBus::new(peer_db_sender);
        let tx_bus_sender = TxBus::new(
            payment_sender,
            data_sender
        );

        let handler = MessageHandler::new(
            addr_bus_sender.clone(),
            tx_bus_sender.clone()
        );
        let peerman = PeerMan::new(handler.clone());

        return NetworkInterface{
            peerman,
            peer_db,
            addr_bus_sender,
            tx_bus_sender,
            filter: BloomFilterState::new(2.0),
            handler: handler.clone()
        };
    }

    pub fn connect(&mut self, max_peers: usize) {
        self._connect(max_peers, MAXIMUM_HANDSHAKE_ATTEMPTS);
    }

    fn _connect(&mut self, max_peers: usize, attempts: usize) {
        let current_peers = self.peerman.clone().get_count();
        let mut peer_threads = vec![];
        let bloom_filter = self.filter.clone().get_filter();

        if (max_peers <= current_peers) || attempts <= 0 {
            self.peerman.clone().remove_count(current_peers - max_peers);
            return;
        }
        for i in 0..MAX_CONCURRENT_HANDSHAKES {
            let ip_tuple = match self.peer_db.lock().unwrap().get() {
                Some((ip, port)) => {
                    let peerman = self.peerman.clone();
                    let bloom = bloom_filter.clone();
                    peer_threads.push(thread::spawn(move || {
                        return peerman.add_peer(ip, port, bloom.clone());
                    }))
                },
                None => break
            };
        }

        let bad_apples = peer_threads
            .into_iter()
            .filter_map(|thread|
                thread.join().expect("Thrown during unwrap peer").err()
            )
            .collect::<Vec<IpTuple>>();

        self.addr_bus_sender
            .send(AddrEvent::Op(AddrOp(bad_apples, AddressAction::Remove)));

        thread::sleep(Duration::from_millis(100));
        self._connect(max_peers, attempts - 1);
    }

    pub fn subscribe_to_payments(&self, key: &Key) {
        self.subscribe_to_comms(key);
        self.tx_bus_sender
            .send(TxEvent::AddPaymentOutput(create_lock_script(&key.pubkeyhash)))
            .unwrap();
    }

    pub fn subscribe_to_comms(&self, key: &Key) {
        self.update_filter(&key.pubkeyhash.0);
    }

    pub fn update_filter(&self, data: &[u8]) {
        self.filter.clone().update_filter(data.clone());

        self.peerman.clone().broadcast(Message::FilterAdd(FilterAdd{
            data: data.to_vec()
        }));
    }

    pub(crate) fn broadcast(&self, tx: Tx) {
        println!("TX: {}", hex::encode(tx.to_bytes()));
        let hash = tx.hash();
        return;
        self.handler.clone().send(tx);
        self.peerman.clone().broadcast(Message::Inv(Inv{
            objects: vec![
                InvVect{
                    obj_type: INV_VECT_TX,
                    hash
                }
            ]
        }));
    }
}



