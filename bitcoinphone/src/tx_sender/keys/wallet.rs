use std::collections::HashMap;
use sv::messages::OutPoint;
use sv::script::Script;
use sv::util::{Hash160, hash160};
use secp256k1::{PublicKey, SecretKey, Secp256k1, All};
use std::{fs, thread};
use rand::OsRng;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize, Serializer};
use sv::address::addr_encode;
use sv::address::AddressType::P2PKH;
use sv::network::Network;
use crate::util::traits::Spawnable;
use std::sync::mpsc::{SyncSender, sync_channel, RecvError};
use crate::util::constants::{PubKeyHash, UTXOPacket, UTXO, Key};
use sv::transaction::p2pkh::create_lock_script;

pub const WALLET_FILE_NAME: &'static str = "wallet.json";

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    keys: Vec<Key>,
    utxos: Vec<UTXO>,
}

pub trait Walletable {
    fn get_key_count(&self) -> usize;
    fn add_utxo(&self, utxo: UTXO);
    fn gen_key(&self, curve: &Secp256k1<All>, rng: &mut OsRng);
    fn get_key(&self, index: usize) -> Key;
    fn get_balance(&self) -> u64;
    fn get_utxo_set(&self, amount: i64) -> Option<Vec<UTXO>>;
}

impl Wallet {
    pub fn new() -> Arc<Mutex<Wallet>> {
        let wallet_file = Self::open_file();
        let wallet = wallet_file.unwrap_or(Wallet{
            keys: vec![],
            utxos: vec![]
        });
        let arced_wallet = Arc::from(Mutex::from(wallet));
        println!("Wallet has {} sats balance", arced_wallet.get_balance());

        return arced_wallet;
    }

    fn open_file() -> Option<Wallet> {
        if fs::metadata(WALLET_FILE_NAME).is_err() {
            return None;
        }

        let file = fs::read_to_string(WALLET_FILE_NAME)
            .expect("Unable to open wallet!");

        return Some(serde_yaml::from_str(&file).unwrap());
    }

    fn update_file(&self) {
        let string = serde_yaml::to_string(self)
            .expect("Unable to serialize as YAML");
        fs::write(WALLET_FILE_NAME, string);
    }

    pub fn get_addr(pubkeyhash: &Hash160) -> String {
        return addr_encode(pubkeyhash, P2PKH, Network::Mainnet);
    }
}

impl Spawnable<UTXOPacket> for Arc<Mutex<Wallet>> {
    fn spawn_gateway(&self) -> SyncSender<UTXOPacket> {
        let (tx, rx) = sync_channel(1000);
        let this = self.clone();
        thread::spawn(move || loop {
            match rx.recv() {
                Ok(packet) => {
                    (packet as Vec<UTXO>)
                        .into_iter()
                        .for_each(|utxo| {
                            println!("Got new utxo: {} sats", utxo.sats);
                            this.add_utxo(utxo);
                        });
                }
                Err(e) => {
                    panic!("Invalid utxo passed to wallet..");
                }
            };
        });
        return tx;
    }
}

impl Walletable for Arc<Mutex<Wallet>> {
    fn gen_key(&self, curve: &Secp256k1<All>, rng: &mut OsRng) {
        let (secret_key, public_key) = curve.generate_keypair(rng);
        let pubkeyhash = hash160(&public_key.serialize());
        let this = self.clone();
        let mut unlocked_this = this.lock().unwrap();
        unlocked_this.keys.push(Key{
            secret_key,
            public_key,
            pubkeyhash
        });
        unlocked_this.update_file();
        println!("Created {} keys", this.get_key_count());
    }

    fn add_utxo(&self, mut utxo: UTXO) {
        let pubkeyhash = utxo.script_pubkey.clone();
        let this = self.clone();
        let key_count = this.get_key_count();

        for index in 0..key_count {
            let key = this.get_key(index);
            if pubkeyhash == create_lock_script(&key.pubkeyhash) {
                utxo.key = key;
            }
        }

        let mut unlocked_this = this.lock().unwrap();
        unlocked_this.utxos.push(utxo);
        unlocked_this.update_file();
    }

    fn get_key_count(&self) -> usize {
        return self.clone().lock().unwrap()
            .keys.len();
    }

    fn get_key(&self, index: usize) -> Key {
        return self.clone().lock().unwrap().keys.get(index)
            .expect("Tried to get bad key")
            .clone();
    }

    fn get_balance(&self) -> u64 {
        return self
            .clone()
            .lock()
            .unwrap()
            .utxos
            .iter()
            .fold(0, |prev, cur| cur.sats as u64 + prev);
    }

    fn get_utxo_set(&self, amount: i64) -> Option<Vec<UTXO>> {
        let mut wallet = self
            .lock()
            .unwrap();

        let mut computed = 0;
        let mut utxo_set = vec![];

        while computed < amount {
            if wallet.utxos.is_empty() {
                break;
            }
            let utxo = wallet.utxos.remove(0);
            computed += utxo.sats;
            utxo_set.push(utxo);
        }

        wallet.update_file();
        return Some(utxo_set);
    }
}