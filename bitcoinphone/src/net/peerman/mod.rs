use std::{thread, time};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender, SyncSender};

use sv::messages::Message;
use sv::peer::Peer;
use sv::util::BloomFilter;

use filter::BloomFilterState;
pub use peer::PPeer;

use crate::net::peer_db::IpTuple;
use crate::net::peerman::message_handler::MessageHandler;

mod peer;
pub mod message_handler;
pub mod filter;

pub struct PeerMan {
    peers: Arc<Mutex<HashMap<String, PPeer>>>,
    handler: Arc<MessageHandler>,
}

impl PeerMan {
    pub(crate) fn new(handler: Arc<MessageHandler>) -> Arc<Self> {
        return Arc::from(PeerMan{
            peers: Arc::from(Mutex::new(HashMap::new())),
            handler
        });
    }

    pub fn get_count(self: Arc<Self>) -> usize {
        return self.peers.lock().unwrap().len();
    }

    pub(crate) fn add_peer(self: Arc<Self>, ip_address: IpAddr, port: u16, bloom_filter: BloomFilter) -> Result<(), IpTuple> {
        let peer = PPeer::new(
            ip_address,
            port,
            self.handler.clone(),
            bloom_filter
        );

        if peer.is_none() {
            return Err((ip_address, port));
        }
        let naked_peer = peer.unwrap();
        self.peers
            .lock()
            .unwrap()
            .insert(naked_peer.get_id(), naked_peer);

        return Ok(());
    }

    pub fn broadcast(self: Arc<Self>, msg: Message) {
        let peers = self.peers
            .lock()
            .unwrap();
        for peer in peers.values() {
            peer.send(&msg);
        }
    }

    pub fn remove_count(self: Arc<Self>, count: usize) {
        let mut peers = self.peers.lock()
            .unwrap();

        let leftovers = peers
            .keys()
            .take(count)
            .map(|a| a.clone())
            .collect::<Vec<String>>();

        leftovers.iter().for_each(|peer_id| {
            let ppeer = peers.remove(peer_id);
            ppeer.unwrap().sv_peer.disconnect();
        })
    }

    fn remove_peer(self: Arc<Self>, peer: Arc<Peer>) {
        let peer_id = PPeer::get_id_from_peer(peer);
        self.peers
            .lock()
            .unwrap()
            .remove(&peer_id);
    }

}

