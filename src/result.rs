#![allow(unused)]
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufWriter;
use std::error::Error;
use std::path::Path;
use csv::Writer;
use serde_json;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AnalyzerResult {
    pub prev_block_height: u32,
    pub prev_block_hash: String,
    pub target_block_height: u32,
    pub target_block_hash: String,
    pub mempool_fee_rate_estimate: f64,
    pub mempool_subset_fee_rate_estimate: f64,
    pub mempool_subset_txns_count: usize,
    pub target_block_txns_count: usize,
    pub mempool_subset_txns_in_target_block_count: usize,
    pub conditional_probability: f64,
    pub mempool_depth: usize,
    pub blocks_found_count: usize,
    pub block_discovery_timestamp: String,
    pub snapshot_timestamp: String
}

pub struct AnalyzerResultUpdate {
    pub target_block_hash: String,
    pub target_block_txns_count: usize,
    pub mempool_subset_txns_in_target_block_count: usize
}

pub trait AnalyzerResultProcessor {
    fn save_intermediate_result(&self, result: Vec<AnalyzerResult>) -> Result<(), Box<dyn Error>>;
    fn update_intermediate_result(&self, prev_block_height: u32, result_update: AnalyzerResultUpdate) -> Result<(), Box<dyn Error>>;
    fn load_intermediate_result(&self) -> Result<Vec<AnalyzerResult>, Box<dyn Error>>;
    fn result_exists(&self) -> bool;
    fn save_final_result(&self, intermediate_result: Vec<AnalyzerResult>) -> Result<(), Box<dyn Error>>;
}

impl AnalyzerResultProcessor for AnalyzerResult {
    fn save_intermediate_result(&self, result: Vec<AnalyzerResult>) -> Result<(), Box<dyn Error>> {
        let file = File::create("result.json")?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &result)?;

        println!("Intermediate result saved in result.json");

        Ok(())
    }

    fn update_intermediate_result(&self, prev_block_height: u32, result_update: AnalyzerResultUpdate) -> Result<(), Box<dyn Error>> {
        let mut result = self.load_intermediate_result()?;

        for i in 0..result.len() {
            if result[i].target_block_height == prev_block_height {
                result[i].target_block_hash = result_update.target_block_hash.clone();
                result[i].target_block_txns_count = result_update.target_block_txns_count;
                result[i].mempool_subset_txns_in_target_block_count = result_update.mempool_subset_txns_in_target_block_count;
                result[i].conditional_probability = result_update.mempool_subset_txns_in_target_block_count as f64 / result[i].mempool_subset_txns_count as f64;
            }
        };

        self.save_intermediate_result(result)?;

        Ok(())
    }

    fn load_intermediate_result(&self) -> Result<Vec<AnalyzerResult>, Box<dyn Error>> {
        let mut result = vec![];
        let file = File::open("result.json")?;
        let reader = std::io::BufReader::new(file);
        let deserialized_data: Vec<AnalyzerResult> = serde_json::from_reader(reader)?;

        for item in deserialized_data {
            result.push(item);
        }

        Ok(result)
    }

    fn result_exists(&self) -> bool {
        Path::new("result.json").exists()
    }

    fn save_final_result(&self, intermediate_result: Vec<AnalyzerResult>) -> Result<(), Box<dyn Error>> {

        let file = File::create("result.csv")?;
        let mut wtr = Writer::from_writer(file);

        wtr.write_record([
            "prev_block_height", "prev_block_hash", "target_block_height", "target_block_hash",
            "mempool_fee_rate_estimate", "mempool_subset_fee_rate_estimate", "mempool_subset_txns_count", 
            "target_block_txns_count", "mempool_subset_txns_in_target_block_count", 
            "conditional_probability", "mempool_depth", "blocks_found_count", 
            "block_discovery_timestamp", "snapshot_timestamp",
        ])?;

        for record in intermediate_result {
            wtr.write_record([
                &record.prev_block_height.to_string(),
                &record.prev_block_hash,
                &record.target_block_height.to_string(),
                &record.target_block_hash,
                &record.mempool_fee_rate_estimate.to_string(),
                &record.mempool_subset_fee_rate_estimate.to_string(),
                &record.mempool_subset_txns_count.to_string(),
                &record.target_block_txns_count.to_string(),
                &record.mempool_subset_txns_in_target_block_count.to_string(),
                &record.conditional_probability.to_string(),
                &record.mempool_depth.to_string(),
                &record.blocks_found_count.to_string(),
                &record.block_discovery_timestamp,
                &record.snapshot_timestamp
            ])?;
        }

        wtr.flush()?;

        println!("Result written to CSV file successfully!");

        Ok(())
    }
}