
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::Arc;
use sv::util::Hash256;
use super::constants::KeyType;
use sv::messages::Tx;
use crate::util::constants::UTXO;
use sv::transaction::sighash::SigHashCache;

pub trait SyncSpawnable<T> {
    fn spawn_gateway(&self) -> SyncSender<T>;
}

pub trait Spawnable<T> {
    fn spawn_gateway(&self) -> SyncSender<T>;
}

pub trait Signatory {
    fn add_signature(
        &self,
        tx: &mut Tx,
        utxo: &UTXO,
        sighash_cache: &mut SigHashCache,
        index: usize
    );
}
