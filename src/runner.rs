#![allow(unused)]
use tokio::time::{self, Duration};
use tokio::sync::Mutex;
use std::clone;
use std::sync::{Arc, MutexGuard};
use std::{error::Error, os::unix::process::parent_id, path::PathBuf, process::Command};
use crate::block_data::{BlockTransaction, BlockMonitor};
use crate::config::Config;
use crate::mempool_data::{self, MempoolTransaction, MempoolData};
use crate::mempool_data_subsets::{
    filter_mempool_txns, 
    HighFeeFilter, 
    InputsCountFilter, 
    LowFeeFilter, 
    MempoolTransactionFilter, 
    OutputsCountFilter
};
use crate::strategies::{FeeRateEstimator, select_strategy};
use std::collections::HashMap;
use std::collections::HashSet;
use crate::config::Commands;
use crate::result::{AnalyzerResult, AnalyzerResultProcessor, AnalyzerResultUpdate};
use chrono::{DateTime, Utc, TimeZone};

#[derive(Debug)]
pub enum AnalyzerError {
    SomethingWentWrong
}

#[derive(Clone)]
pub struct Runner<'a> {
    pub strategy: &'a dyn FeeRateEstimator,
}

impl<'a> Runner<'a> {
    pub fn new(strategy: &'a dyn FeeRateEstimator) -> Self {
        Runner { strategy }
    }
}

fn get_timestamp() -> String {
    let now: DateTime<Utc> = Utc::now();
    let formatted_time = now.format("%Y-%m-%d %H:%M:%S%z").to_string();
    formatted_time
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
pub struct MempoolFilterParams<'a> {
    config: Config,
    runner: Runner<'a>,
    threshold: f64,
    mempool_txns: Vec<MempoolTransaction>
}

pub fn fetch_mempool_txns_subset<'a>(filter_params: MempoolFilterParams<'a>) -> Result<(Vec<MempoolTransaction>, f64), Box<dyn Error>> 
{
    let mut filtered_txns = vec![];
    let mut subset_fee_rate_estimate= 0.0;

    match &filter_params.config.commands {
        Commands::SubsetSample {
            low_fee_txns,
            high_fee_txns ,
            inputs,
            outputs,
        } => {

            let mut filters: Vec<Box<dyn MempoolTransactionFilter>> = Vec::new();

            if *low_fee_txns {
                filters.push(Box::new(LowFeeFilter { threshold: filter_params.threshold }));
            }

            if *high_fee_txns {
                filters.push(Box::new(HighFeeFilter { threshold: filter_params.threshold }));
            }

            if let Some((comparison, count)) = inputs {
                filters.push(Box::new(InputsCountFilter { comparison: comparison.clone(), count: *count }));
            }

            if let Some((comparison, count)) = outputs {
                filters.push(Box::new(OutputsCountFilter { comparison: comparison.clone(), count: *count }));
            }

            filtered_txns = filter_mempool_txns(&filter_params.mempool_txns, &filters);
            subset_fee_rate_estimate = filter_params.runner.strategy.estimate_fee_rate(&filtered_txns);
        }
    }

    Ok((filtered_txns, subset_fee_rate_estimate))
}


pub async fn run_analysis(
    config: Config, 
    last_snapshot: Arc<Mutex<SnapshotData>>,
    mempool_txns: Vec<MempoolTransaction>
) -> Result<AnalyzerResult, Box<dyn Error + Send + Sync>> {

    let last_snapshot = last_snapshot.lock().await;
    
    let mut result = vec![];

    let strategy = select_strategy(&config.strategy_name);
    let runner = Runner::new(&*strategy);
    
    let mut target_block_txns: Vec<BlockTransaction> = vec![];
    let mut filtered_txns_in_block: Vec<MempoolTransaction> = vec![];

    let prev_block_height = BlockMonitor::get_prev_block_height().expect("Error getting prev block height");
    let target_block_height = prev_block_height + 1;

    let mut blocks_found_count = last_snapshot.analyzer_result.blocks_found_count;

   if check_if_target_block_found(last_snapshot.target_block_height, target_block_height) {
        
        println!("Target Block Found!");

        let fee_rate_estimate = runner.strategy.estimate_fee_rate(&last_snapshot.mempool_txns);

        let filter_params = MempoolFilterParams {
            config: config.clone(),
            runner: runner.clone(),
            threshold: fee_rate_estimate,
            mempool_txns: last_snapshot.mempool_txns.clone()
        };
    
        let (filtered_txns, subset_fee_rate_estimate) = fetch_mempool_txns_subset(filter_params).expect("Error fetching mempool txns subset");

        let target_block_hash = BlockMonitor::get_block_hash(last_snapshot.target_block_height).expect("Error getting block hash");
        let target_block_txns = BlockMonitor::get_target_block_txns(last_snapshot.target_block_height).expect("Error getting target block txns");
        let filtered_txns_in_block = find_common_transactions (
            &filtered_txns, 
            &target_block_txns
        );

        blocks_found_count = blocks_found_count + 1;

        let mut analyzer_result = AnalyzerResult {
            prev_block_height: last_snapshot.analyzer_result.prev_block_height,
            prev_block_hash: last_snapshot.analyzer_result.prev_block_hash.clone(),
            target_block_height: last_snapshot.analyzer_result.target_block_height,
            target_block_hash,
            mempool_fee_rate_estimate: last_snapshot.analyzer_result.mempool_fee_rate_estimate,
            mempool_subset_fee_rate_estimate: last_snapshot.analyzer_result.mempool_subset_fee_rate_estimate,
            mempool_subset_txns_count: filtered_txns.len(),
            target_block_txns_count: target_block_txns.len(),
            mempool_subset_txns_in_target_block_count: filtered_txns_in_block.len(),
            conditional_probability: filtered_txns_in_block.len() as f64 / filtered_txns.len() as f64,
            mempool_depth: last_snapshot.mempool_txns.len(),
            blocks_found_count,
            block_discovery_timestamp: get_timestamp(),
            snapshot_timestamp: last_snapshot.analyzer_result.snapshot_timestamp.clone()
        };

        if analyzer_result.result_exists() {
            result = analyzer_result.load_intermediate_result().expect("Error loading intermediate result");
        }

        let item_already_exists = result.iter().any(|r| r.prev_block_height == last_snapshot.analyzer_result.prev_block_height);
    
        if !item_already_exists {
            result.push(analyzer_result.clone());
            analyzer_result.save_intermediate_result(result);
        }
    }

    let fee_rate_estimate = runner.clone().strategy.estimate_fee_rate(&mempool_txns);

    let filter_params = MempoolFilterParams {
        config,
        runner,
        threshold: fee_rate_estimate,
        mempool_txns: mempool_txns.clone()
    };
    
    let (filtered_txns, subset_fee_rate_estimate) = fetch_mempool_txns_subset(filter_params)
    .expect("Error fetching mempool txns subset");

    let prev_block_height = BlockMonitor::get_prev_block_height().expect("Error getting prev block height");
    let prev_block_hash = BlockMonitor::get_block_hash(prev_block_height).expect("Error getting block hash");
    let target_block_height = prev_block_height + 1;
    let mut target_block_hash = "".to_string();

    let mut analyzer_result = AnalyzerResult {
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
        mempool_depth: last_snapshot.mempool_txns.len(),
        blocks_found_count,
        block_discovery_timestamp: "".to_string(),
        snapshot_timestamp: get_timestamp()
    };

    Ok(analyzer_result)
}

fn check_if_target_block_found(last_target_block_height: u32, target_block_height: u32) -> bool {
    last_target_block_height < target_block_height
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

#[derive(Debug)]
pub struct SnapshotData {
    target_block_height: u32,
    mempool_txids: HashSet<String>,
    mempool_txns: Vec<MempoolTransaction>,
    analyzer_result: AnalyzerResult
}

async fn fetch_current_mempool_txns() -> Result<(Vec<MempoolTransaction>, HashSet<String>), Box<dyn Error>> {
    let mut mempool_txns: Vec<MempoolTransaction> = vec![];

    let raw_mempool_data: Vec<u8> = bcli(&format!("getrawmempool true")).expect("Error getting raw mempool");
    let mempool_data_str = String::from_utf8(raw_mempool_data).expect("Failed to convert bytes to string");
    let mempool_data: HashMap<String, MempoolData> = serde_json::from_str(&mempool_data_str).expect("Could not deserialize mempool data");
        
    mempool_txns = MempoolTransaction::fetch_mempool_txns(&mempool_data).expect("Could not fetch mempool txns");
    let mut mempool_txids: HashSet<String> = HashSet::new();

    for txn in mempool_txns.clone() {
        mempool_txids.insert(txn.txid);
    }

    Ok((mempool_txns, mempool_txids))
}

pub async fn run_tasks(config: Config) -> Result<(), Box<dyn Error>> {
    let interval = Duration::from_secs(1);
    let mut ticker = time::interval(interval);

    let (mempool_txns, mempool_txids) = fetch_current_mempool_txns().await.expect("Could not fetch current mempool txns");

    let last_snapshot = Arc::new(Mutex::new(SnapshotData {
        target_block_height: BlockMonitor::get_latest_target_block().expect("Could not get latest target block"),
        mempool_txids: mempool_txids.clone(),
        mempool_txns: mempool_txns.clone(),
        analyzer_result: AnalyzerResult::default()
    }));

    loop {
        ticker.tick().await;

        let config_clone = config.clone();
        let last_snapshot_clone: Arc<Mutex<SnapshotData>> = last_snapshot.clone();

        let (mempool_txns, mempool_txids) = fetch_current_mempool_txns()
        .await
        .expect("Could not fetch current mempool txns");

        tokio::spawn(async move {

            match run_analysis(config_clone, last_snapshot_clone.clone(), mempool_txns).await {
                Ok(analyzer_result) => {

                    let mut snapshot = last_snapshot_clone.lock().await;
                    
                    let target_block_height = BlockMonitor::get_latest_target_block()
                    .unwrap_or(snapshot.target_block_height);

                    let (mempool_txns, mempool_txids) = fetch_current_mempool_txns()
                        .await
                        .unwrap_or((snapshot.mempool_txns.clone(), snapshot.mempool_txids.clone()));

                    snapshot.target_block_height = target_block_height;
                    snapshot.mempool_txids = mempool_txids.clone();
                    snapshot.mempool_txns = mempool_txns.clone();
                    snapshot.analyzer_result = analyzer_result;
                }
                Err(e) => {
                    eprintln!("Error in run_analysis: {}", e);
                }
            }
        });

        let last_snapshot_main_thread_clone = last_snapshot.clone();
        let last_snapshot_main_thread_clone_mut = last_snapshot_main_thread_clone.lock().await;

        let config_main_thread_clone = config.clone();

        println!(
            "last_snapshot target & timestamp: {:?} & {}",
            last_snapshot_main_thread_clone_mut.target_block_height, 
            last_snapshot_main_thread_clone_mut.analyzer_result.snapshot_timestamp
        );
        
        if last_snapshot_main_thread_clone_mut.analyzer_result.blocks_found_count >= config_main_thread_clone.duration {
            println!("Analysis duration reached. Exiting...");
            break;
        }
    }   

    Ok(())
}