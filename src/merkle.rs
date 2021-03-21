use anyhow::Result;
use bitcoin::{
    hashes::{hex::ToHex, Hash},
    TxMerkleNode, Txid,
};
use serde_json::{json, Value};

pub(crate) struct Proof {
    proof: Vec<TxMerkleNode>,
    pos: usize,
    height: usize,
}

impl Proof {
    pub(crate) fn create(txid: Txid, txids: &[Txid], height: usize) -> Result<Self> {
        let mut pos = match txids.iter().position(|current_txid| *current_txid == txid) {
            None => bail!("missing tx {} at block {}", txid, height),
            Some(pos) => pos,
        };
        let mut hashes: Vec<TxMerkleNode> = txids
            .iter()
            .map(|txid| TxMerkleNode::from_hash(txid.as_hash()))
            .collect();

        let mut proof = vec![];
        while hashes.len() > 1 {
            if hashes.len() % 2 != 0 {
                let last = *hashes.last().unwrap();
                hashes.push(last);
            }
            pos = if pos % 2 == 0 { pos + 1 } else { pos - 1 };
            proof.push(hashes[pos]);
            pos /= 2;
            hashes = hashes
                .chunks(2)
                .map(|pair| {
                    let left = pair[0];
                    let right = pair[1];
                    let input = [&left[..], &right[..]].concat();
                    TxMerkleNode::hash(&input)
                })
                .collect()
        }
        Ok(Self { proof, pos, height })
    }

    pub(crate) fn to_value(&self) -> Value {
        let merkle: Vec<String> = self.proof.iter().map(|node| node.to_hex()).collect();

        json!({"block_height": self.height, "pos": self.pos, "merkle": merkle})
    }
}
