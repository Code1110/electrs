use anyhow::{Context, Result};

use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter::FromIterator;
use std::ops::Bound;

use bitcoin::hashes::Hash;
use bitcoin::{Amount, OutPoint, Transaction, Txid};
use bitcoincore_rpc::{json, Client, RpcApi};
use rayon::prelude::*;

use crate::types::ScriptHash;

pub(crate) struct Entry {
    pub txid: Txid,
    pub tx: Transaction,
    pub fee: Amount,
    pub vsize: u64,
    pub has_unconfirmed_inputs: bool,
}

/// Mempool current state
pub(crate) struct Mempool {
    entries: HashMap<Txid, Entry>,
    by_funding: BTreeSet<(ScriptHash, Txid)>,
    by_spending: BTreeSet<(OutPoint, Txid)>,

    txid_min: Txid,
    txid_max: Txid,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            entries: Default::default(),
            by_funding: Default::default(),
            by_spending: Default::default(),

            txid_min: Txid::from_inner([0x00; 32]),
            txid_max: Txid::from_inner([0xFF; 32]),
        }
    }

    pub(crate) fn get(&self, txid: &Txid) -> Option<&Entry> {
        self.entries.get(txid)
    }

    pub(crate) fn filter_by_funding(&self, scripthash: &ScriptHash) -> Vec<&Entry> {
        let range = (
            Bound::Included((*scripthash, self.txid_min)),
            Bound::Included((*scripthash, self.txid_max)),
        );
        self.by_funding
            .range(range)
            .map(|(_, txid)| self.get(txid).expect("missing funding mempool tx"))
            .collect()
    }

    pub(crate) fn filter_by_spending(&self, outpoint: &OutPoint) -> Vec<&Entry> {
        let range = (
            Bound::Included((*outpoint, self.txid_min)),
            Bound::Included((*outpoint, self.txid_max)),
        );
        self.by_spending
            .range(range)
            .map(|(_, txid)| self.get(txid).expect("missing spending mempool tx"))
            .collect()
    }

    pub fn sync(&mut self, rpc: &Client) -> Result<()> {
        let txids = rpc.get_raw_mempool().context("failed to get mempool")?;
        debug!("loading {} mempool transactions", txids.len());

        let new_txids = HashSet::<Txid>::from_iter(txids);
        let old_txids = HashSet::<Txid>::from_iter(self.entries.keys().copied());

        let to_add = &new_txids - &old_txids;
        let to_remove = &old_txids - &new_txids;

        let removed = to_remove.len();
        for txid in to_remove {
            self.remove_entry(txid);
        }
        let entries: Vec<_> = to_add
            .par_iter()
            .filter_map(|txid| {
                match (
                    rpc.get_raw_transaction(txid, None),
                    rpc.get_mempool_entry(txid),
                ) {
                    (Ok(tx), Ok(entry)) => Some((txid, tx, entry)),
                    _ => None,
                }
            })
            .collect();
        let added = entries.len();
        for (txid, tx, entry) in entries {
            self.add_entry(*txid, tx, entry);
        }
        debug!(
            "{} mempool txs: {} added, {} removed",
            self.entries.len(),
            added,
            removed,
        );
        Ok(())
    }

    fn add_entry(&mut self, txid: Txid, tx: Transaction, entry: json::GetMempoolEntryResult) {
        for txi in &tx.input {
            self.by_spending.insert((txi.previous_output, txid));
        }
        for txo in &tx.output {
            let scripthash = ScriptHash::new(&txo.script_pubkey);
            self.by_funding.insert((scripthash, txid)); // may have duplicates
        }
        let entry = Entry {
            txid,
            tx,
            vsize: entry.vsize,
            fee: entry.fees.base,
            has_unconfirmed_inputs: !entry.depends.is_empty(),
        };
        assert!(
            self.entries.insert(txid, entry).is_none(),
            "duplicate mempool txid"
        );
    }

    fn remove_entry(&mut self, txid: Txid) {
        let entry = self.entries.remove(&txid).expect("missing tx from mempool");
        for txi in entry.tx.input {
            self.by_spending.remove(&(txi.previous_output, txid));
        }
        for txo in entry.tx.output {
            let scripthash = ScriptHash::new(&txo.script_pubkey);
            self.by_funding.remove(&(scripthash, txid)); // may have misses
        }
    }
}
