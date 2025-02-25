#![allow(unused)]
use tokio::time::{self, Duration};
use std::{error::Error, os::unix::process::parent_id, path::PathBuf, process::Command};
use crate::block_data::{BlockTransaction, BlockMonitor};
use crate::config::Config;
use crate::mempool_data::{self, MempoolTransaction, MempoolData};
use crate::mempool_data_subsets::{filter_mempool_txns, HighFeeFilter, InputsCountFilter, LowFeeFilter, MempoolTransactionFilter, OutputsCountFilter};
use crate::strategies::{FeeRateEstimator, select_strategy};
use std::collections::HashMap;
use std::collections::HashSet;
use crate::config::Commands;
use crate::result::{AnalyzerResult, AnalyzerResultProcessor, AnalyzerResultUpdate};

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

    pub fn run_analysis(
        &self,
        current_target_block_height: &mut u32,
        config: Config, 
        mempool_txns: Vec<MempoolTransaction>
    ) -> Result<(), Box<dyn Error>> {
        
        let mut result = vec![];
        let fee_rate_estimate = self.strategy.estimate_fee_rate(&mempool_txns);

        let mut filtered_txns = vec![];
        let mut subset_fee_rate_estimate= 0.0;

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
                subset_fee_rate_estimate = self.strategy.estimate_fee_rate(&filtered_txns);
            }
        }

        let prev_block_height = BlockMonitor::get_prev_block_height()?;
        let prev_block_hash = BlockMonitor::get_block_hash(prev_block_height)?;
        let target_block_height = prev_block_height + 1;
        let mut target_block_hash = "".to_string();
        
        let mut target_block_txns: Vec<BlockTransaction> = vec![];
        let mut filtered_txns_in_block: Vec<MempoolTransaction> = vec![];

        let analyzer_result: AnalyzerResult = AnalyzerResult {
            prev_block_height,
            prev_block_hash,
            target_block_height,
            target_block_hash,
            mempool_fee_rate_estimate: fee_rate_estimate,
            mempool_subset_fee_rate_estimate: subset_fee_rate_estimate,
            mempool_subset_txns_count: filtered_txns.len(),
            target_block_txns_count: target_block_txns.len(),
            mempool_subset_txns_in_target_block_count: filtered_txns_in_block.len(),
            conditional_probability: filtered_txns_in_block.len() as f64 / filtered_txns.len() as f64,
            mempool_depth: mempool_txns.len()
        };

        if analyzer_result.result_exists() {
            result = analyzer_result.load_intermediate_result()?;
        }

        result.push(analyzer_result.clone());

        analyzer_result.save_intermediate_result(result);

        if *current_target_block_height < target_block_height {
            println!("current target block mined!");

            let target_block_hash = BlockMonitor::get_block_hash(prev_block_height)?;
            let target_block_txns = BlockMonitor::get_target_block_txns(prev_block_height)?;
            let filtered_txns_in_block = find_common_transactions(&filtered_txns, &target_block_txns);

            let result_update = AnalyzerResultUpdate {
                target_block_hash,
                target_block_txns_count: target_block_txns.len(),
                mempool_subset_txns_in_target_block_count: filtered_txns_in_block.len() 
            };

            analyzer_result.update_intermediate_result(prev_block_height, result_update);
        }

        *current_target_block_height = target_block_height;

        let now_system = std::time::SystemTime::now();
        let since_the_epoch = now_system
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards");

        Ok(())
    }
}

fn find_common_transactions(
    filtered_txns: &[MempoolTransaction],
    txns_in_block: &[BlockTransaction],
) -> Vec<MempoolTransaction> {
    let txids_in_block: HashSet<&String> = txns_in_block.iter().map(|tx| &tx.txid).collect();

    filtered_txns
        .iter()
        .filter(|tx| txids_in_block.contains(&tx.txid))
        .cloned()
        .collect()
}

pub async fn run_strategy(current_target_block_height: &mut u32, config: Config) -> Result<(), Box<dyn Error>> {
    
    let strategy = select_strategy(&config.strategy_name);
    let runner = Runner::new(&*strategy);

    let mut mempool_txns: Vec<MempoolTransaction> = vec![];

    let raw_mempool_data: Vec<u8> = bcli(&format!("getrawmempool true")).expect("Error getting raw mempool");
    let mempool_data_str = String::from_utf8(raw_mempool_data).expect("Failed to convert bytes to string");
    let mempool_data: HashMap<String, MempoolData> = serde_json::from_str(&mempool_data_str).expect("Could not deserialize mempool data");
        
    mempool_txns = MempoolTransaction::fetch_mempool_txns(&mempool_data)?;
    let _ = runner.run_analysis(current_target_block_height, config, mempool_txns);

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

    let mut current_target_block_height = BlockMonitor::get_initial_target_block().expect("Cound not get intial target block");
        
    loop {
        ticker.tick().await;

        run_strategy(&mut current_target_block_height, config.clone()).await;
    }
}