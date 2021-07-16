use std::thread;
use std::sync::mpsc::SyncSender;
use crate::util::constants::{DataPacket, UIEvent, PaymentKey, PubKeyHash, CommunicationsKey};
use crate::tx_sender::keys::KeyManager;
use sv::address::{addr_encode, AddressType, addr_decode};
use sv::network::Network;
use sv::util::Hash160;

mod events;

pub fn start(key_manager: KeyManager, sender: SyncSender<DataPacket>) {
    thread::spawn(move || {
        let funding_address = get_address(&key_manager.get_key(PaymentKey).pubkeyhash);
        let comms_address = get_address(&key_manager.get_key(CommunicationsKey).pubkeyhash);
        println!("Welcome to Bitphone, please fund this address {}", funding_address);
        println!("Your personal communication address is {}", comms_address);
        let mut address = String::new();
        std::io::stdin().read_line(&mut address)
            .expect("unable to read from input");

        let output = get_pubkeyhash(address).0.to_vec();
        sender.send(DataPacket::UIEvent(UIEvent::Start{
            output
        }));
    });
}




fn get_address(pubkeyhash: &PubKeyHash) -> String {
    return addr_encode(
        &pubkeyhash,
        AddressType::P2PKH,
        Network::Mainnet
    );
}
fn get_pubkeyhash(address: String) -> Hash160 {
    return addr_decode(
        &address.trim(),
        Network::Mainnet
    ).expect("Unable to get pubkeyhash").0;
}