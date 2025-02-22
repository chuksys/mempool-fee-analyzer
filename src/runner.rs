#![allow(unused)]
use std::error::Error;
use crate::config::Config;
use crate::mempool_data::{self, MempoolTransaction, MempoolData};
use crate::strategies::{FeeEstimator, select_strategy};
use std::path::Path;
use std::collections::HashMap;

pub struct Runner<'a> {
    pub strategy: &'a dyn FeeEstimator,
}

impl<'a> Runner<'a> {

    pub fn new(strategy: &'a dyn FeeEstimator) -> Self {
        Runner { strategy }
    }

    pub fn run_analysis(&self, mempool_data: Vec<MempoolTransaction>) -> Result<(), Box<dyn Error>> {
        
        let fee_estimate = self.strategy.estimate_fee(mempool_data);

        //based on user config input, select mempool subset relative to fee estimate (for low, high fee txns) or mempool txns that meet input/output requirements. 
        //save txids from subset for evaluation later
        //find fee estimate of subset and log in csv
        //repeat process every minute

        //monitor target block confirmation
        //Upon confirmation, fetch all txns included in the target block which were part of subset
        //find the median of these txns included in target block which were part of subset
        //log this median in the csv in columns that correspond with earlier logged medians from subset. 

        Ok(())
    }
}

pub fn run_strategy(config: Config) -> Result<(), Box<dyn Error>> {
    
    let strategy = select_strategy(&config.strategy_name);
    let runner = Runner::new(&*strategy);

    let mut mempool_txns: Vec<MempoolTransaction> = vec![];

    //temporary mempool for development
    let file_path = "mempool.json";

    if Path::new(file_path).exists() {
        match MempoolData::load_from_file(file_path) {
            Ok(mempool_data) => {
                mempool_txns = MempoolTransaction::fetch_mempool_txns(&mempool_data)?;
            }
            Err(err) => {
                println!("Failed to load mempool from cache: {}", err);
            }
        }
    } else {
        let raw_mempool_data: Vec<u8> = mempool_data::bcli(&format!("getrawmempool true")).expect("Error getting raw mempool");
        let mempool_data_str = String::from_utf8(raw_mempool_data).expect("Failed to convert bytes to string");
        let mempool_data: HashMap<String, MempoolData> = serde_json::from_str(&mempool_data_str).expect("Could not deserialize mempool data");
        
        let _ = MempoolData::save_to_file(&mempool_data, &file_path)?;
        mempool_txns = MempoolTransaction::fetch_mempool_txns(&mempool_data)?;
    }

    let _ = runner.run_analysis(mempool_txns);

    Ok(())
}