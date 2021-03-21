use bitcoin::{Transaction, Txid};

use std::collections::HashMap;

use std::sync::{Arc, RwLock};

pub struct Cache {
    txs: Arc<RwLock<HashMap<Txid, Transaction>>>,
}

impl Cache {
    pub fn new() -> Self {
        let txs = Arc::new(RwLock::new(HashMap::new()));
        Self { txs }
    }

    pub(crate) fn add_tx(&self, txid: Txid, f: impl FnOnce() -> Transaction) {
        self.txs.write().unwrap().entry(txid).or_insert_with(f);
    }

    pub(crate) fn get_tx<F, T>(&self, txid: &Txid, f: F) -> Option<T>
    where
        F: FnOnce(&Transaction) -> T,
    {
        self.txs.read().unwrap().get(txid).map(f)
    }
}
