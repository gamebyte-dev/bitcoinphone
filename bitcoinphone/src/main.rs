#![allow(warnings)]
use domain::Domain;
use crate::util::traits::Spawnable;
use crate::net::NetworkInterface;
use std::sync::mpsc::{sync_channel, RecvError};
use crate::ui::start;
use crate::util::constants::DataPacket;
use crate::tx_sender::keys::{Wallet, KeyManager};
use crate::tx_sender::TxSender;

mod domain;
mod phone;
mod net;
mod ui;
mod util;
mod tx_sender;

fn main() {
    let wallet = Wallet::new();
    let payment_sender = wallet.spawn_gateway();
    let (data_sender, data_receiver) = sync_channel(1000);

    let wallet = Wallet::new();
    // Start the wallet.
    let wallet_sender = wallet.spawn_gateway();

    // Start the tx_sender.
    let key_manager = KeyManager::new(wallet.clone());
    let tx_sender = TxSender::new(
        key_manager.clone(),
        payment_sender,
        data_sender.clone()
    );

    // Start the UI sender.
    let ui_sender = start(key_manager.clone(), data_sender.clone());
    let packet = data_receiver.recv()
        .expect("Got an error in data receiver");

    let mut domain = Domain::new(
        tx_sender.clone(),
        data_receiver,
        key_manager.clone()
    );

    domain.start_processing(packet);

    //domain.run_phone();

}



/*
mod phone;
mod tx_factory;
mod net;
mod traits;
mod key_factory;

fn main () {

    tx_factory::TxGateway::new();

    let (sender, mic_receiver): (SyncSender<DataPacket>, Receiver<DataPacket>) = sync_channel(1000);
    tx_gateway.update_data_sender(sender.clone());

    let speaker_sender: SyncSender<DataPacket> = phone::Phone::new(PhoneConfig{
        sample_rate: 8000.0,
        frames_per_buffer: 8000,
        jitter_delay_nanos: 1000
    }, sender);

    let tx_gateway_clone = tx_gateway.clone();
    thread::spawn(move || loop {
        match mic_receiver.recv() {
            Ok(data) => {
                println!("Sending mic data to tx tx_sender!");
                tx_gateway_clone.send(data);
            }
            Err(e) => {
                panic!("Weird mic error {}", e);
            }
        }
    });

    loop {
        match network_data_receiver.recv() {
            Ok(data) => {
                speaker_sender.send(data).unwrap();
            },
            Err(e) => {
                panic!("Error receiving mic data {}", e);
            }
        }
    }
}*/