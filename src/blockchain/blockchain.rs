use super::GENESIS;
use super::hash::{H256};
use std::collections::{HashMap};

pub struct BlockChain {
    pub height: usize,
    pub chain: HashMap<usize, H256>, 
    pub latest_hash: H256,

}

impl BlockChain {
    pub fn new() -> BlockChain {
        BlockChain {
            height: 0,
            chain: HashMap::new(),
            latest_hash: GENESIS,
        } 
    }

    pub fn insert(&mut self, hash: H256) {
        let height = &mut self.height;
        self.chain.insert(*height, hash);
        *height += 1;
        self.latest_hash = hash;
    }

    pub fn get_height(&self) -> usize {
        self.height 
    }

    pub fn get_latest_block_hash(&self) -> H256 {
        self.latest_hash 
    }
}
