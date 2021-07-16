use sv::script::Script;
use crate::util::constants::{PHONE_PREFIX, UTXO, PaymentKey, PubKeyHash};
use sv::messages::{Tx, TxIn, TxOut};
use crate::util::traits::Signatory;
use sv::transaction::sighash::{sighash, SigHashCache, SIGHASH_ANYONECANPAY, SIGHASH_ALL, SIGHASH_FORKID};
use sv::transaction::p2pkh::{create_lock_script, create_unlock_script};
use crate::tx_sender::MIN_DUST;

pub const TX_FIXED_SIZE: i64 = 8;
pub const P2PKH_INPUT_SIZE: usize = 157;
pub const SATS_PER_KB: i64 = 500;

#[derive(Default)]
pub struct TxBuilder {
    total_funded_value: i64,
    bytes_added: i64,
    used_amount: i64,
    outputs: Vec<(Script, i64)>,
    change_output: Option<Script>,
    locktime: u32
}

impl TxBuilder {
    pub fn new(locktime: u32, total_funded_value: i64) -> TxBuilder {
        return TxBuilder{
            total_funded_value,
            bytes_added: TX_FIXED_SIZE,
            outputs: vec![],
            change_output: None,
            used_amount: 0,
            locktime
        }
    }

    pub fn add_data_output(mut self, mut data: Vec<u8>, amount: i64) -> Self {
        let mut data_ouput = PHONE_PREFIX.to_vec();
        data_ouput.append(&mut data);
        return self.add_script_output(Script(data_ouput), amount);
    }

    pub fn add_script_output(mut self, script: Script, amount: i64) -> Self {
        return self.add_output(script, Some(amount));
    }

    pub fn add_change_output(mut self, pubkeyhash: PubKeyHash) -> Self {
        return self.add_output(
            create_lock_script(&pubkeyhash), None);
    }

    fn add_output(mut self, script: Script, amount: Option<i64>) -> Self {
        self.bytes_added += script.0.len() as i64;
        self.bytes_added += 21; // 8 satoshi bytes + 4 locktime bytes + 9 varint bytes (overestimate)

        match amount {
            Some(amount) => {
                self.outputs.push((script, amount));
                self.used_amount += amount;
            },
            None => {
                self.change_output = Some(script);
            }
        }

        return self;
    }

    pub fn build(
        mut self,
        inputs: &mut Vec<UTXO>,
        signatory: impl Signatory
    ) -> Tx {
        self.bytes_added += (inputs.len() * P2PKH_INPUT_SIZE) as i64;

        let mut tx = self.construct_tx_without_inputs();
        Self::add_inputs_and_sign(&mut tx, inputs, signatory);

        return tx;
    }

    pub fn construct_tx_without_inputs(self) -> Tx {
        let mut tx = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: self.locktime
        };

        tx.outputs = self.outputs
            .into_iter()
            .map(|(script, amount)| TxOut{
                satoshis: amount,
                lock_script: script
            })
            .collect::<Vec<TxOut>>();

        if self.change_output.is_none() {
            println!("Danger: Building tx without change output");
            return tx;
        }

        let miner_fees = (self.bytes_added * SATS_PER_KB + SATS_PER_KB) / 1000;
        let remainder = self.total_funded_value - self.used_amount - miner_fees;
        if remainder < MIN_DUST {
            return tx;
        }

        tx.outputs.push(self.change_output.map(move |script| TxOut{
            satoshis: remainder,
            lock_script: script
        }).unwrap());

        return tx;
    }

    fn add_inputs_and_sign(tx: &mut Tx, inputs: &mut Vec<UTXO>, signatory: impl Signatory) {
        let mut sighash_cache = SigHashCache::new();
        for (index, utxo) in inputs.iter_mut().enumerate() {
            tx.inputs.push(TxIn {
                prev_output: utxo.outpoint.clone(),
                unlock_script: Script::new(),
                sequence: utxo.sequence,
            });
            utxo.sequence += 1;

            signatory.add_signature(tx, utxo, &mut sighash_cache, index);
        }
    }
}