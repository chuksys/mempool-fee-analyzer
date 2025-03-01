#![allow(unused)]
use tokio::time::{self, Duration};
use tokio::sync::Mutex;
use std::clone;
use std::sync::Arc;
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

fn find_common_txids(vec1: &[String], vec2: &[String]) -> Vec<String> {
    // Create a HashSet from vec2 for efficient lookups.
    let set2: HashSet<&String> = vec2.iter().collect();

    // Iterate over vec1 and check if each txid is in set2.
    vec1.iter()
        .filter(|txid| set2.contains(txid))
        .cloned() // Clone the txids to create a new Vec<String>.
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
) -> Result<AnalyzerResult, Box<dyn Error>> {

    let last_snapshot = last_snapshot.lock().await;

    println!("last_snapshot_target {:?}", last_snapshot.target_block_height);
    
    let mut result = vec![];

    let strategy = select_strategy(&config.strategy_name);
    let runner = Runner::new(&*strategy);
    
    let mut target_block_txns: Vec<BlockTransaction> = vec![];
    let mut filtered_txns_in_block: Vec<MempoolTransaction> = vec![];

    let prev_block_height = BlockMonitor::get_prev_block_height()?;
    let target_block_height = prev_block_height + 1;

    let mut analyzer_result: AnalyzerResult = AnalyzerResult::default();

   if check_if_target_block_found(last_snapshot.target_block_height, target_block_height) {
        
        println!("Target Block Found!");

        let fee_rate_estimate = runner.strategy.estimate_fee_rate(&last_snapshot.mempool_txns);

        let filter_params = MempoolFilterParams {
            config,
            runner,
            threshold: fee_rate_estimate,
            mempool_txns: last_snapshot.mempool_txns.clone()
        };
    
        let (filtered_txns, subset_fee_rate_estimate) = fetch_mempool_txns_subset(filter_params)?;

        let target_block_hash = BlockMonitor::get_block_hash(last_snapshot.target_block_height)?;
        let target_block_txns = BlockMonitor::get_target_block_txns(last_snapshot.target_block_height)?;
        let filtered_txns_in_block = find_common_transactions (
            &filtered_txns, 
            &target_block_txns
        );

        analyzer_result = AnalyzerResult {
            prev_block_height: last_snapshot.analyzer_result.prev_block_height,
            prev_block_hash: last_snapshot.analyzer_result.prev_block_hash.clone(),
            target_block_height: last_snapshot.analyzer_result.target_block_height,
            target_block_hash: last_snapshot.analyzer_result.target_block_hash.clone(),
            mempool_fee_rate_estimate: last_snapshot.analyzer_result.mempool_fee_rate_estimate,
            mempool_subset_fee_rate_estimate: last_snapshot.analyzer_result.mempool_subset_fee_rate_estimate,
            mempool_subset_txns_count: filtered_txns.len(),
            target_block_txns_count: target_block_txns.len(),
            mempool_subset_txns_in_target_block_count: filtered_txns_in_block.len(),
            conditional_probability: filtered_txns_in_block.len() as f64 / filtered_txns.len() as f64,
            mempool_depth: last_snapshot.mempool_txns.len()
        };

        if analyzer_result.result_exists() {
            result = analyzer_result.load_intermediate_result()?;
        }
    
        result.push(analyzer_result.clone());
        analyzer_result.save_intermediate_result(result);

    } else {

        let fee_rate_estimate = runner.strategy.estimate_fee_rate(&mempool_txns);

        let filter_params = MempoolFilterParams {
            config,
            runner,
            threshold: fee_rate_estimate,
            mempool_txns: mempool_txns.clone()
        };
    
        let (filtered_txns, subset_fee_rate_estimate) = fetch_mempool_txns_subset(filter_params)?;

        let prev_block_height = BlockMonitor::get_prev_block_height()?;
        let prev_block_hash = BlockMonitor::get_block_hash(prev_block_height)?;
        let target_block_height = prev_block_height + 1;
        let mut target_block_hash = "".to_string();

        analyzer_result = AnalyzerResult {
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
            mempool_depth: last_snapshot.mempool_txns.len()
        };
    }

    //last_snapshot.block_height = target_block_height;

    /*let now_system = std::time::SystemTime::now();
    let since_the_epoch = now_system
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards"); */

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

/*     let last_snapshot = Arc::new(Mutex::new(SnapshotData {
        target_block_height: BlockMonitor::get_latest_target_block().expect("Could not get latest target block"),
        mempool_txids: mempool_txids.clone(),
        mempool_txns: mempool_txns.clone()
    })); */

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
        let last_snapshot_clone = last_snapshot.clone();

        let (mempool_txns, mempool_txids) = fetch_current_mempool_txns().await.expect("Could not fetch current mempool txns");

        tokio::spawn(async move {

            let analyzer_result = run_analysis(config_clone, last_snapshot_clone, mempool_txns).await.expect("Could not get analyzer result");

            let target_block_height = BlockMonitor::get_latest_target_block().expect("Could not get latest target block");
            let (mempool_txns, mempool_txids) = fetch_current_mempool_txns().await.expect("Could not fetch current mempool txns");

            let last_snapshot = Arc::new(Mutex::new(SnapshotData {
                target_block_height,
                mempool_txids: mempool_txids.clone(),
                mempool_txns: mempool_txns.clone(),
                analyzer_result
            }));

            {
                let mut snapshot = last_snapshot.lock().await;
                snapshot.target_block_height = target_block_height;
                snapshot.mempool_txids = mempool_txids;
                snapshot.mempool_txns = mempool_txns;
            }
        });
    }

    Ok(())
}