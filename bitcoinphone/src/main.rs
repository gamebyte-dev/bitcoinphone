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
//mod phone;
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
