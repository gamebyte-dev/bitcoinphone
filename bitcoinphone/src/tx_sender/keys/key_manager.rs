use secp256k1::{Message};
use secp256k1::{PublicKey, SecretKey, Secp256k1, All};
use rand::OsRng;

use sv::util::{Hash160, hash160, BloomFilter, Hash256};
use sv::address::{AddressType, addr_encode, addr_decode};
use sv::network::Network;
use sv::script::Script;
use sv::transaction::p2pkh::{create_lock_script, create_unlock_script};
use std::collections::HashMap;


use super::wallet::{Wallet, Walletable};
use std::sync::{Mutex, Arc};
use sv::transaction::generate_signature;
use crate::util::constants::{KeyType, PaymentKey, CommunicationsKey, PubKeyHash, UTXO, Key};
use crate::util::traits::Signatory;
use sv::messages::{Tx, TxIn};
use sv::transaction::sighash::{SigHashCache, sighash, SIGHASH_ANYONECANPAY, SIGHASH_FORKID, SIGHASH_ALL};


#[derive(Clone)]
pub struct KeyManager {
    rng: OsRng,
    curve: Secp256k1<All>,
    key_map: HashMap<KeyType, Key>,

    wallet: Arc<Mutex<Wallet>>,
}

impl KeyManager {
    pub fn new(wallet: Arc<Mutex<Wallet>>) -> KeyManager {
        let mut manager = KeyManager{
            rng: OsRng::new().expect("Unable to initialize RNG!"),
            curve: Default::default(),
            key_map: HashMap::new(),
            wallet
        };
        manager.setup_keys();

        return manager;
    }

    pub fn get_key(&self, key_type: KeyType) -> Key {
        return self.key_map
            .get(&key_type)
            .unwrap()
            .clone();
    }

    pub fn setup_keys(&mut self) {
        while self.wallet.get_key_count() < 2 {
            self.wallet.gen_key(&self.curve, &mut self.rng);
        }

        self.key_map.insert(
            CommunicationsKey,
            self.wallet.get_key(0)
        );

        self.key_map.insert(
            PaymentKey,
            self.wallet.get_key(1)
        );
    }
}

pub const SIGHASH_TYPE: u8 = SIGHASH_ALL | SIGHASH_ANYONECANPAY | SIGHASH_FORKID;

impl Signatory for &KeyManager {

    fn add_signature(
        &self,
        tx: &mut Tx,
        utxo: &UTXO,
        sighash_cache: &mut SigHashCache,
        index: usize
    ) {
        let sighash = sighash(
            tx,
            index,
            utxo.script_pubkey.0.as_slice(),
            utxo.sats,
            SIGHASH_TYPE,
            sighash_cache
        ).expect("Unable to create sighash");
        let signature = generate_signature(
            utxo.key.secret_key.as_ref(),
            &sighash,
            SIGHASH_TYPE
        ).expect("Unabled to generate signature");

        let input = tx.inputs.get_mut(index)
            .expect("Invalid inputs");

        input.unlock_script.append_data(signature.as_slice());
        input.unlock_script.append_data(&utxo.key.public_key.serialize());
    }
}
