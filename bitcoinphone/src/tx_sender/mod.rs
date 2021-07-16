use serde::Serialize;
use std::sync::mpsc::{Sender, SyncSender, sync_channel, RecvError};
use sv::messages::{Tx, OutPoint, TxIn, TxOut, Payload};
use std::collections::HashMap;
use sv::script::Script;
use std::thread;
use std::sync::{Arc, Mutex, RwLock};
use crate::net::{TxOperation, NetworkInterface};
use sv::transaction::p2pkh::create_unlock_script;
use crate::util::constants::{UTXO, DataPacket, CommunicationsKey, UTXOPacket, PaymentKey};
use crate::util::traits::Spawnable;
use tx_builder::TxBuilder;
use crate::tx_sender::keys::{KeyManager, Wallet, Walletable};
use crate::util::get_timestamp;
use std::ops::{Add, AddAssign};
use std::time::Duration;

mod tx_builder;
pub mod keys;

pub const MIN_DUST: i64 = 500;
pub const P2PKH_OUTPUT_SIZE: usize = 256;
pub const SATS_PER_KB: i32 = 500;
pub const MAX_BYTES_PER_PACKET: usize = 50000;
pub const MAXIMUM_PEERS: usize = 8;

pub struct TxSender {
    active_utxos: RwLock<Vec<UTXO>>,
    network_interface: NetworkInterface,
    pub key_manager: KeyManager,
    total_funding_amount: RwLock<i64>,
    expected_locktime: u32
}

impl TxSender {
    pub fn new(
        key_manager: KeyManager,
        payment_sender: SyncSender<UTXOPacket>,
        data_sender: SyncSender<DataPacket>,
    ) -> Arc<TxSender> {
        let mut network_interface = NetworkInterface::new(
            payment_sender,
            data_sender
        );

        println!("Attempting to connect to {}, peers..", MAXIMUM_PEERS);
        network_interface.connect(MAXIMUM_PEERS);
        network_interface.subscribe_to_payments(&key_manager.get_key(PaymentKey));
        network_interface.subscribe_to_comms(&key_manager.get_key(CommunicationsKey));
        println!("Connected to peers!");

        return Arc::from(TxSender{
            active_utxos: RwLock::from(Vec::new()),
            network_interface,
            key_manager,
            total_funding_amount: RwLock::from(0),
            expected_locktime: get_timestamp()
                .add(Duration::from_secs(7200))
                .as_secs() as u32
        });
    }

    pub fn get_utxos(self: Arc<Self>) {
        let utxo_set = self.key_manager.wallet
            .get_utxo_set(MAX_BYTES_PER_PACKET as i64);

        if utxo_set.is_none() {
            println!("You need to fund your address!");
            return;
        }

        self.clone().active_utxos.write().unwrap().append(&mut utxo_set.unwrap());
        let total = self.clone().active_utxos.read().unwrap().iter()
            .fold(0, |prev, cur| prev + cur.sats);

        self.total_funding_amount
            .write()
            .unwrap()
            .add_assign(total);
    }

    pub fn finalize(self: Arc<Self>, metadata: impl Serialize, output: Script) {
        let funding = *self.total_funding_amount.read().unwrap();
        let mut inputs = self.active_utxos.write().unwrap();
        let data = bincode::serialize(&metadata)
            .expect("Unable to serialize packet data.");

        let mut tx = TxBuilder::new(self.expected_locktime, funding)
            .add_data_output(data,0)
            .add_change_output(self.key_manager.get_key(PaymentKey).pubkeyhash)
            .build(&mut inputs, &self.key_manager);

        self.network_interface.broadcast(tx);
    }

    pub fn send_data(self: Arc<Self>, data: impl Serialize, receiver_output: Script) {
        let funding = *self.total_funding_amount.read().unwrap();
        let mut inputs = self.active_utxos.write().unwrap();
        let mut data = bincode::serialize(&data)
            .expect("Unable to serialize packet data.");

        let mut tx = TxBuilder::new(self.expected_locktime, funding)
            .add_data_output(data,0)
            .add_script_output(receiver_output, MIN_DUST)
            .add_change_output(self.key_manager.get_key(PaymentKey).pubkeyhash)
            .build(&mut inputs, &self.key_manager);

        self.network_interface.broadcast(tx);
    }
}