use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, RecvTimeoutError, sync_channel, SyncSender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sv::script::Script;

use crate::{util};
use crate::net::NetworkInterface;
//use crate::phone::PhoneConfig;
use crate::tx_sender::TxSender;
use crate::util::constants::{CommunicationsKey, DataPacket, UIEvent, PaymentKey, Key};
use crate::util::traits::Spawnable;
use crate::tx_sender::keys::KeyManager;
use sv::transaction::p2pkh::create_lock_script;

pub const SYNC_CLICKS: u64 = 5;

pub struct Domain {
    tx_sender: Arc<TxSender>,
    network_receiver: Receiver<DataPacket>,
    key_manager: KeyManager,
    peer_address: Script,
    jitter: u64,
}

impl Domain {
    pub fn new(
        tx_sender: Arc<TxSender>,
        network_receiver: Receiver<DataPacket>,
        key_manager: KeyManager
    ) -> Domain {
        return Domain {
            tx_sender: tx_sender,
            network_receiver,
            peer_address: Script(vec![]),
            key_manager,
            jitter: 0
        };
    }

    pub fn start_processing(&mut self, packet: DataPacket) {
        self.tx_sender.clone().get_utxos();
        match packet {
            DataPacket::UIEvent(UIEvent::Start{ output }) => {
                self.peer_address = Script(output);
                self.run_sender();
                println!("running phone!");
            }
            DataPacket::Start { output, sync_count } => {
                self.peer_address = Script(output);
                self.run_receiver(sync_count);
                println!("running phone!");
            }
            _ => {
                println!("Received invalid starting packet.")
            }
        }
    }

    fn run_receiver(&mut self, sync_count: u64) {
        self.tx_sender
            .clone()
            .send_data(
                DataPacket::StartAck{
                    output: self.get_comms_output(),
                    sync_count: SYNC_CLICKS,
                },
                self.peer_address.clone()
            );
        let jitter = self.wait_sync(sync_count);
        self.start_sync(SYNC_CLICKS);
    }

    fn run_sender(&mut self) {
        self.tx_sender
            .clone()
            .send_data(
                DataPacket::Start{
                    output: self.get_comms_output(),
                    sync_count: SYNC_CLICKS,
                },
                self.peer_address.clone()
            );

        let data_packet = self.network_receiver.recv_timeout(Duration::from_secs(10))
            .expect("Errored out waiting for startACK");

        if let DataPacket::StartAck{sync_count, ..} = data_packet.clone() {
            self.start_sync(SYNC_CLICKS);
            let jitter = self.wait_sync(sync_count);
        }  else {
            panic!("Expected start-ack got something else {:?} ", data_packet);
        }
    }

    fn start_sync(&self, max_syncs: u64) {
        println!("Starting syncing algorithm.");
        for count in (0..max_syncs).rev() {
            println!("Sending sync count={}", count);
            self.tx_sender.clone().send_data(DataPacket::Sync{
                time: util::get_timestamp().as_nanos(),
                count,
            }, self.peer_address.clone());
            thread::sleep(Duration::from_millis(250));
        }
    }

    fn wait_sync(&self, expected_syncs: u64) -> u64 {
        println!("Starting waiting algorithm.");
        let mut timeouts = vec![];

        loop {
            match self.network_receiver.recv_timeout(Duration::from_secs(10)) {
                Ok(packet) => match packet {
                    DataPacket::Sync { time, count } => {
                        println!("Got sync count={}", count);
                        timeouts.push(util::get_timestamp());
                        if count == 0 {
                            break;
                        }
                    }
                    _ => {
                        panic!("Wrong type of packet received.");
                    }
                }
                Err(_) => {
                    panic!("Timed out waiting for next packet");
                }
            }
        }

        let jitter = Self::get_jitter(timeouts, expected_syncs + 1);
        println!("Calculated network jitter: {} ms", jitter);

        return 2 * jitter;
    }
/*
    pub fn run_phone(&mut self, jitter_delay_nanos: u64) {
        // Move tx_sender out of the struct since we need it in a seperate thread.
        let moved_gateway = std::mem::replace(&mut self.tx_gateway, None)
            .unwrap();
        let address = self.connection_address;

        let (mic_sender, mic_receiver) = sync_channel(1000);
        let speaker_sender = phone::Phone::new(PhoneConfig{
            sample_rate: 8000.0,
            frames_per_buffer: 8000,
            jitter_delay_nanos
        }, mic_sender);

        thread::spawn(move || loop {
            match mic_receiver.recv() {
                Ok(packet) => {
                    moved_gateway
                        .send_data(packet, address)
                },
                Err(_) => {
                    panic!("Got error from mic!");
                }
            }
        });


        loop {
            match self.network_receiver.recv() {
                Ok(data) => {
                    speaker_sender.send(data).unwrap();
                },
                Err(e) => {
                    panic!("Error receiving mic data {}", e);
                }
            }
        }
    }*/

    fn get_jitter(mut jitters: Vec<Duration>, count: u64) -> u64 {
        let mut prev = jitters.remove(0);
        let mut delta_sum = Duration::from_secs(0);
        for current in jitters {
            delta_sum += current - prev;
            prev = current;
        }

        return (delta_sum.as_millis() as u64 / (count - 1));
    }

    fn get_comms_output(&self) -> Vec<u8> {
        let Key{pubkeyhash, ..} = self.key_manager.get_key(PaymentKey);

        return create_lock_script(&pubkeyhash).0;
    }
}