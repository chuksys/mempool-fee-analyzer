#![allow(unused)]
use tokio::time::{self, Duration};
use std::{error::Error, os::unix::process::parent_id, path::PathBuf, process::Command};
use crate::block_data;
use crate::config::Config;
use crate::mempool_data::{self, MempoolTransaction, MempoolData};
use crate::mempool_data_subsets::{filter_mempool_txns, HighFeeFilter, InputsCountFilter, LowFeeFilter, MempoolTransactionFilter, OutputsCountFilter};
use crate::strategies::{FeeRateEstimator, select_strategy};
use std::path::Path;
use std::collections::HashMap;
use crate::config::Commands;

#[derive(Debug)]
pub enum AnalyzerError {
    SomethingWentWrong
}

pub struct Runner<'a> {
    pub strategy: &'a dyn FeeRateEstimator,
}

impl<'a> Runner<'a> {

    pub fn new(strategy: &'a dyn FeeRateEstimator) -> Self {
        Runner { strategy }
    }

    pub fn run_analysis(&self, config: Config, mempool_txns: Vec<MempoolTransaction>) -> Result<(), Box<dyn Error>> {
        
        let fee_rate_estimate = self.strategy.estimate_fee_rate(&mempool_txns);

        let mut filtered_txns = vec![];

        match &config.commands {
            Commands::SubsetSample {
                low_fee_txns,
                high_fee_txns,
                inputs,
                outputs,
            } => {

                let mut filters: Vec<Box<dyn MempoolTransactionFilter>> = Vec::new();
    
                if *low_fee_txns {
                    filters.push(Box::new(LowFeeFilter { threshold: fee_rate_estimate }));
                }
    
                if *high_fee_txns {
                    filters.push(Box::new(HighFeeFilter { threshold: fee_rate_estimate }));
                }
    
                if let Some((comparison, count)) = inputs {
                    filters.push(Box::new(InputsCountFilter { comparison: comparison.clone(), count: *count }));
                }
    
                if let Some((comparison, count)) = outputs {
                    filters.push(Box::new(OutputsCountFilter { comparison: comparison.clone(), count: *count }));
                }
    
                filtered_txns = filter_mempool_txns(&mempool_txns, &filters);
    
                //println!("Filtered Transactions: {:?}", filtered_txns.len());
            }
        }

        let prev_block_height = block_data::BlockMonitor::get_prev_block_height()?;
        let prev_block_hash = block_data::BlockMonitor::get_block_hash(prev_block_height)?;

        println!("----------------------------");
        println!("prev_block_height {:?}", prev_block_height);
        println!("prev_block_hash {:?}", prev_block_hash);
        println!("target_block_height {:?}", prev_block_height + 1);
        println!("----------------------------");

        //save txids from subset for evaluation later
        //find fee_rate estimate of subset and log in csv (log other important metrics too e.g prev_block_height, target_block_height)
        
        //log the CBlockPolicyEstimator fee_rate estimate for each mempool-based estimate logged
        //repeat process every minute

        //monitor target block confirmation
        //Upon confirmation, fetch all txns included in the target block which were part of subset
        //find the median of these txns included in target block which were part of subset
        //log this median in the csv in columns that correspond with earlier logged medians from subset (log target_block_hash too) 

        Ok(())
    }
}

pub async fn run_strategy(config: Config) -> Result<(), Box<dyn Error>> {
    
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
        let raw_mempool_data: Vec<u8> = bcli(&format!("getrawmempool true")).expect("Error getting raw mempool");
        let mempool_data_str = String::from_utf8(raw_mempool_data).expect("Failed to convert bytes to string");
        let mempool_data: HashMap<String, MempoolData> = serde_json::from_str(&mempool_data_str).expect("Could not deserialize mempool data");
        
        let _ = MempoolData::save_to_file(&mempool_data, &file_path)?;
        mempool_txns = MempoolTransaction::fetch_mempool_txns(&mempool_data)?;
    }

    let _ = runner.run_analysis(config, mempool_txns);

    Ok(())
}

pub fn bcli(cmd: &str) -> Result<Vec<u8>, AnalyzerError> {
    let mut args = vec![];
    args.extend(cmd.split(' '));

    let result = Command::new("bitcoin-cli")
        .args(&args)
        .output()
        .map_err(|_| AnalyzerError::SomethingWentWrong)?;

    if result.status.success() {
        return Ok(result.stdout);
    } else {
        return Ok(result.stderr);
    }
}

pub async fn run_tasks(config: Config) {
    let interval = Duration::from_secs(60);
    let mut ticker = time::interval(interval);

    loop {
        ticker.tick().await;

        let config_clone = config.clone();
        run_strategy(config_clone).await;
    }
}