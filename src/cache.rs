use bitcoin::{Block, BlockHash, Transaction, Txid};

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct Cache {
    txs: Arc<RwLock<HashMap<Txid, Transaction>>>,
    txids: Arc<RwLock<HashMap<BlockHash, Vec<Txid>>>>,
}

impl Cache {
    pub fn new() -> Self {
        let txs = Arc::new(RwLock::new(HashMap::new()));
        let txids = Arc::new(RwLock::new(HashMap::new()));
        Self { txs, txids }
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

    pub(crate) fn add_txids(&self, blockhash: BlockHash, block: &Block) {
        self.txids
            .write()
            .unwrap()
            .entry(blockhash)
            .or_insert_with(|| block.txdata.iter().map(|tx| tx.txid()).collect());
    }

    pub(crate) fn get_txids<F, T>(&self, blockhash: &BlockHash, f: F) -> Option<T>
    where
        F: FnOnce(&[Txid]) -> T,
    {
        self.txids
            .read()
            .unwrap()
            .get(blockhash)
            .map(|txids| f(&txids))
    }
}
