use std::time::Duration;
use serde::{Serialize, Deserialize};
use sv::script::Script;
use sv::messages::OutPoint;
use sv::util::Hash160;
use sv::script::op_codes::{OP_FALSE, OP_RETURN};
use secp256k1::{SecretKey, PublicKey};

pub const MAX_INTERVALS: u64 = 10;
pub const DEFAULT_INTERVALS: u64 = 5;
pub const SYNC_INTERVAL: Duration = Duration::from_millis(500);

pub const SAMPLE_RATE: f64 = 44_100.0;
pub const FRAMES_PER_BUFFER: u32 = 64;
pub const CHANNELS: i32 = 1;

pub const BUFFER_SIZE: usize = SAMPLE_RATE as usize;
pub const PHONE_PREFIX: &[u8] = &[OP_FALSE, OP_RETURN, 0x70, 0x68, 0x6f, 0x6e, 0x65];

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum UIEvent {
    Start{
        output: Vec<u8>
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DataPacket {
    UIEvent(UIEvent),
    Start {
        output: Vec<u8>,
        sync_count: u64
    },
    StartAck {
        output: Vec<u8>,
        sync_count: u64
    },
    Sync {
        time: u128,
        count: u64
    },
    Data {
        counter: u32,
        buffer: Vec<u8>
    }
}

type Address = Script;
type Sequence = u32;
type Amount = u64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Key {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
    pub pubkeyhash: PubKeyHash
}

impl Key {
    pub fn new() -> Key {
        let secret_key = SecretKey::from_slice(&[
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE,
            0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B,
            0xBF, 0xD2, 0x5E, 0x8C, 0xD0, 0x36, 0x41, 0x40,
        ]).unwrap();
        let public_key = PublicKey::from_slice(&[3, 23, 183, 225, 206, 31, 159, 148, 195, 42, 67, 115, 146, 41, 248, 140, 11, 3, 51, 41, 111, 180, 110, 143, 114, 134, 88, 73, 198, 174, 52, 184, 78]).unwrap();
        return Key {
            secret_key: secret_key.clone(),
            public_key: public_key,
            pubkeyhash: Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UTXO {
    pub outpoint: OutPoint,
    pub sats: i64,
    pub key: Key,
    pub script_pubkey: Script,
    pub sequence: u32
}

pub type UTXOPacket = Vec<UTXO>;
pub type PubKeyHash = Hash160;

pub type KeyType = u8;

pub const CommunicationsKey: KeyType = 0;
pub const PaymentKey: KeyType = 1;
