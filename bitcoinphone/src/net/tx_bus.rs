use std::thread;
use std::net::IpAddr;
use sv::messages::{OutPoint, Tx, Message, Addr};
use std::sync::mpsc::{Sender, Receiver, channel, RecvError, SyncSender, sync_channel};
use std::sync::{Arc, RwLock};
use sv::script::op_codes::{OP_FALSE, OP_RETURN};
use sv::script::Script;
use std::time::{SystemTime, UNIX_EPOCH};
use sv::util::Hash160;
use crate::util::constants::{PHONE_PREFIX, UTXO, UTXOPacket, DataPacket, Key};

unsafe fn HAS_PHONE_PREFIX(val: Vec<u8>) -> bool {
    if val.len() <= PHONE_PREFIX.len() {
        return false;
    }

    return (*val.get_unchecked(0) == PHONE_PREFIX[0])
        && (*val.get_unchecked(1) == PHONE_PREFIX[1])
        && (*val.get_unchecked(2) == PHONE_PREFIX[2])
        && (*val.get_unchecked(3) == PHONE_PREFIX[3])
        && (*val.get_unchecked(4) == PHONE_PREFIX[4])
        && (*val.get_unchecked(5) == PHONE_PREFIX[5])
        && (*val.get_unchecked(6) == PHONE_PREFIX[6]);
}

#[derive(Debug)]
pub enum TxOperation {
    Data(Vec<u8>),
    Funding(Vec<UTXO>),
}

pub enum TxEvent {
    RawTx(Tx),
    AddPaymentOutput(Script),
}

pub struct TxBus {
    outputs: RwLock<Vec<Script>>,
    funding_sender: SyncSender<UTXOPacket>,
    data_sender: SyncSender<DataPacket>,
}

impl TxBus {
    pub fn new(
        funding_sender: SyncSender<UTXOPacket>,
        data_sender: SyncSender<DataPacket>
    ) -> SyncSender<TxEvent> {
        let bus = Arc::from(TxBus {
            outputs: RwLock::new(Vec::new()),
            funding_sender,
            data_sender
        });

        return bus.spawn();
    }

    fn spawn(self: Arc<TxBus>) -> SyncSender<TxEvent> {
        let (sender, receiver) = sync_channel(10000);
        std::thread::spawn(move || loop {
            let this = self.clone();
            unsafe {
                match receiver.recv() {
                    Ok(TxEvent::RawTx(tx)) => {
                        this.process_txs(tx);
                    }
                    Ok(TxEvent::AddPaymentOutput(output)) => {
                        this.outputs
                            .write()
                            .unwrap()
                            .push(output);
                    }
                    _ => {
                        panic!("Invalid message passed to bus!");
                    }
                }
            }
        });

        return sender;
    }

    pub unsafe fn process_txs(self: Arc<TxBus>, tx: Tx) {
        let registered_outputs = self.outputs
            .read()
            .unwrap();
        let mut utxos = vec![];

        for (index, output) in tx.outputs.iter().enumerate() {
            let output_vec = output.clone().lock_script.0;
            if HAS_PHONE_PREFIX(output_vec.clone()) {
                let packet_try = bincode::deserialize(&output_vec[PHONE_PREFIX.len()..]);
                if packet_try.is_err() {
                    println!("Got garbled message, dropping");
                    continue;
                }

                self.data_sender
                    .send(packet_try.unwrap())
                    .unwrap();
            }

            for registered_output in &*registered_outputs {
                if output_vec == registered_output.0 {
                    utxos.push(UTXO {
                        outpoint: OutPoint { hash: tx.hash(), index: index as u32 },
                        sats: output.satoshis as i64,
                        key: Key::new(),
                        script_pubkey: Script(output_vec.clone()),
                        sequence: 0
                    });
                }
            }
        }

        if !utxos.is_empty() {
            self.funding_sender
                .send(utxos)
                .unwrap();
        }
    }

    fn parse_p2pkh(script_slice: Vec<u8>) -> Hash160 {
        let mut hashed = Hash160::default();
        hashed.0.clone_from_slice(&script_slice[3..23]);

        return hashed;
    }
}
