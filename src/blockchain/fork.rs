use super::block::{Block};
use super::hash::{H256};
use std::collections::{HashSet, HashMap};
use petgraph::{self};
use petgraph::graph::{NodeIndex};
use petgraph::stable_graph::{StableGraph};

pub struct ForkBuffer {
    pub graph: StableGraph::<Block, ()>, 
    pub hash_node: HashMap<H256, NodeIndex>,
}

impl ForkBuffer {
    pub fn new() -> ForkBuffer {
        ForkBuffer {
            graph: StableGraph::<Block, ()>::new(),
            hash_node: HashMap::new(),
        } 
    }

    pub fn insert(&mut self, block: &Block) {
        let block_hash = block.header.hash;

        match self.hash_node.get(&block_hash) {
            None => {
                let node = self.graph.add_node(block.clone());
                self.hash_node.insert(block_hash, node);
                let prev_hash = block.header.prev_hash;
                match self.hash_node.get(&prev_hash) {
                    Some(prev_node) => {
                        self.graph.add_edge(*prev_node, node, ()); 
                    },
                    None => (),
                }
            },
            _ => (),
        } 
    }

    pub fn remove(&mut self, hash: H256) {
         
    }
}
