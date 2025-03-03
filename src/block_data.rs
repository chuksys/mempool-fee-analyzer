#![allow(unused)]
#![allow(non_snake_case)]
use std::{error::Error, os::unix::process::parent_id, path::PathBuf, process::Command};
use hex::FromHex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::ops::Index;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use crate::mempool_data::{self, MempoolTransaction};
use crate::runner::bcli;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Vin {
    txinwitness: Option<Vec<String>>,
    vout: Option<u32>,
    txid: Option<String>
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Vout {
    n: u32,
    value: f64
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BlockTransaction {
    pub txid: String,
    vin: Vec<Vin>,
    vout: Vec<Vout>
}

#[derive(Deserialize, Serialize, Debug)]
struct Block {
    tx: Vec<BlockTransaction>
}

#[derive(Debug, Clone)]
pub struct BlockMonitor {
    prev_block_height: u32,
    prev_block_hash: String,
    target_block_height: u32,
    target_block_hash: String
}

impl BlockMonitor {
    pub fn get_prev_block_height() -> Result<u32, Box<dyn Error>> {
        let raw_block_height: Vec<u8> = bcli(&format!("getblockcount")).expect("Error getting previous block height");
        let block_height_str = String::from_utf8(raw_block_height).expect("Failed to convert bytes to string");
        let block_height: u32 = block_height_str.trim().parse().unwrap_or_else(|err| {
            println!("Error parsing block height: {}", err);
            0
        });
        
        Ok(block_height)
    }

    pub fn get_block_hash(block_height: u32) -> Result<String, Box<dyn Error>> {
        let raw_block_hash: Vec<u8> = bcli(&format!("getblockhash {}", block_height)).expect("Error getting block hash");
        let block_hash_str = String::from_utf8(raw_block_hash).expect("Failed to convert bytes to string");
        
        Ok(block_hash_str.trim().to_string())
    }

    pub fn get_target_block_txns(target_block_height: u32) -> Result<Vec<BlockTransaction>, Box<dyn Error>> {
        let mut block_transactions: Vec<BlockTransaction> = Vec::new();

        let raw_block_hash: Vec<u8> = bcli(&format!("getblockhash {}", target_block_height)).expect("Error getting block hash");
        let block_hash_str = String::from_utf8(raw_block_hash).expect("Failed to convert bytes to string");

        let raw_block: Vec<u8> = bcli(&format!("getblock {} 2", block_hash_str.trim())).expect("Could not get block");
        let block_str = String::from_utf8(raw_block).expect("Failed to convert bytes to string");

        let block_data: Block = serde_json::from_str(&block_str).expect("Could not deserialize block data");

        for txn in block_data.tx {
            block_transactions.push(txn);
        }
        
        Ok(block_transactions)
    }

    pub fn get_latest_target_block() -> Result<u32, Box<dyn Error>> {
        let raw_block_count: Vec<u8> = bcli(&format!("getblockcount")).expect("Error getting block count");
        let block_count_str = String::from_utf8(raw_block_count).expect("Failed to convert bytes to string");

        let block_count: u32 = block_count_str.trim().parse().unwrap_or_else(| err | {
            println!("Error parsing block count: {}", err);
            0
        });

        let latest_target_block: u32 = block_count + 1;

        Ok(latest_target_block)
    }
}

#[derive(Debug, Clone)]
pub struct BlockMetrics {
    pub total_txns_included_in_block: u32,
    pub total_txns_in_mempool: u32,
    pub total_weight: u64,
    pub total_fees: u64,
    pub total_possible_fees: u64,
    pub percentage_of_total_possible_fees: f64 
}

impl BlockMetrics {
    pub fn calculate_total_block_fees_to_receive(&self, block_txns: Vec<MempoolTransaction>) -> Result<u64, Box<dyn Error>> {
        let mut fees = vec![];

        for txn in block_txns {
            fees.push(txn.fee);
        }

        let total_fees: u64 = fees.iter().sum();
        Ok(total_fees)
    }

    
    pub fn calculate_total_possible_fees(&self, txns: Vec<MempoolTransaction>) -> Result<u64, Box<dyn Error>> {
        let mut fees = vec![];

        for txn in txns {
            fees.push(txn.fee);
        }

        let total_fees: u64 = fees.iter().sum();
        Ok(total_fees)
    }

    pub fn calculate_total_block_weight(&self, block_txns: Vec<MempoolTransaction>) -> Result<u64, Box<dyn Error>> {
        let mut weights = vec![];

        for txn in block_txns {
            weights.push(txn.weight);
        }

        let total_weight: u64 = weights.iter().sum();
        Ok(total_weight)
    }
}

pub struct BlockBuilder {
    mempool_txns_graph: DiGraph<MempoolTransaction, ()>,
    txns: Vec<MempoolTransaction>,
    block_metrics: BlockMetrics 
}

impl BlockBuilder {

    pub fn build_block(mempool_txns: &Vec<MempoolTransaction>) -> Result<Vec<MempoolTransaction>, Box<dyn Error>> {
        
        let mut block_metrics = BlockMetrics {
            total_txns_included_in_block: 0,
            total_txns_in_mempool: mempool_txns.len() as u32,
            total_weight: 0,
            total_fees: 0,
            total_possible_fees: 0,
            percentage_of_total_possible_fees: 0.0
        };
    
        let mut block_builder: BlockBuilder = BlockBuilder {
            mempool_txns_graph: DiGraph::new(),
            txns: mempool_txns.clone(),
            block_metrics: block_metrics.clone()
        };

        //calculate total possible fees to measure fee maximization
        let total_possible_fees = block_metrics.calculate_total_possible_fees(mempool_txns.clone()).expect("Oops!");
        block_metrics.total_possible_fees = total_possible_fees;

        //sort transactions by transaction value (fee_rate) - Fee / Weight Unit - to maximize fees
        let _ = BlockBuilder::sort_txns_by_fee_rate(&mut block_builder);

        //sort transactions in topological order
        let sorted_txns = BlockBuilder::sort_txns_by_ancestor_dependencies(&mut block_builder).expect("Error Fetching Sorted Txns");
        
        //select transactions to be included in block without exceeding max block weight
        let block_txns = BlockBuilder::select_txns_to_be_included_in_block(&mut block_builder, sorted_txns).expect("Error Fetching Block Txns");
        block_metrics.total_txns_included_in_block = block_txns.len() as u32;

        //calculate total fees
        let total_fees = block_metrics.calculate_total_block_fees_to_receive(block_txns.clone()).expect("Error Calculating Total Fees Included In Block");
        block_metrics.total_fees = total_fees;
        
        let percentage_of_total_possible_fees = (total_fees as f64 / total_possible_fees as f64) * 100.0;
        block_metrics.percentage_of_total_possible_fees = percentage_of_total_possible_fees;

        //calculate total weight
        let total_weight = block_metrics.calculate_total_block_weight(block_txns.clone()).expect("Error Calculating Total Block Weight");
        block_metrics.total_weight = total_weight;
        
        Ok(block_txns)
    }

    //this prioritizes txns based on fee_rate (fee / weight). Fee rate tells us the value / unit of data of each txn
    pub fn sort_txns_by_fee_rate(block_builder: &mut BlockBuilder) -> Result<(), Box<dyn Error>> {
        block_builder.txns.sort_by(| a, b | {
            if a.fee_rate.is_nan() || b.fee_rate.is_nan() {
                Ordering::Equal
            } else {
                a.fee_rate.partial_cmp(&b.fee_rate).expect("Could not sort by fee_rate")
            }
        });
        Ok(())
    }

    //sort txns topologically - all ancestor txns must appear before descendants
    pub fn sort_txns_by_ancestor_dependencies(block_builder: &mut BlockBuilder) -> Result<Vec<MempoolTransaction>, Box<dyn Error>> {
        let mut graph = block_builder.mempool_txns_graph.clone();
        let mut node_indices = HashMap::new();

        //create nodes/vertices for all transactions
        for (i, tx) in block_builder.txns.iter().enumerate() {
            let node_idx = graph.add_node(tx.clone());
            node_indices.insert(&tx.txid, node_idx);
        }

        //create edges between parents and children
        for tx in block_builder.txns.iter() {
            let node_idx = node_indices[&tx.txid];
            for parent_txid in &tx.parent_txids {
                if let Some(&parent_node_idx) = node_indices.get(parent_txid) {
                    graph.add_edge(parent_node_idx, node_idx, ());
                }
            }
        }

        //sort nodes in topological order
        let sorted_nodes = toposort(&graph, None).expect("Graph has cycles!");
        let sorted_txns: Vec<MempoolTransaction> = sorted_nodes.into_iter().map(|node_index| {
            graph.index(node_index).clone()
        }).collect();
        Ok(sorted_txns)
    }

    pub fn select_txns_to_be_included_in_block(block_builder: &mut BlockBuilder, sorted_txns: Vec<MempoolTransaction>) -> Result<Vec<MempoolTransaction>, Box<dyn Error>> {
        let mut current_block_weight = block_builder.block_metrics.total_weight;
        let max_block_weight = 4_000_000;
        let mut block_txns: Vec<MempoolTransaction> = vec![];

        for txn in sorted_txns.iter() {
            if current_block_weight + txn.weight <= max_block_weight {
                block_txns.push(txn.clone());
                current_block_weight += txn.weight;
            }
        }
        Ok(block_txns)
    }
}