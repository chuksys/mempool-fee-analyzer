use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufWriter;
use std::error::Error;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub mempool_depth: usize
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
}

impl AnalyzerResultProcessor for AnalyzerResult {
    fn save_intermediate_result(&self, result: Vec<AnalyzerResult>) -> Result<(), Box<dyn Error>> {
        let file = File::create("result.json")?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &result)?;

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
}