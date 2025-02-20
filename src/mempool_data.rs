#![allow(unused)]
use std::{error::Error, os::unix::process::parent_id, path::PathBuf, process::Command};
use hex::FromHex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

#[derive(Debug)]
pub enum AnalyzerError {
    SomethingWentWrong
}

fn bcli(cmd: &str) -> Result<Vec<u8>, AnalyzerError> {
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

#[derive(Serialize, Deserialize, Debug)]
struct Fees {
    ancestor: f64,
    base: f64,
    descendant: f64,
    modified: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransactionData {
    ancestorcount: u32,
    ancestorsize: u32,
    #[serde(rename = "bip125-replaceable")]
    bip125_replaceable: bool,
    depends: Vec<String>,
    descendantcount: u32,
    descendantsize: u32,
    fees: Fees,
    height: u32,
    spentby: Vec<String>,
    time: u64,
    unbroadcast: bool,
    vsize: u32,
    weight: u32,
    wtxid: String,
}

#[derive(Debug, Deserialize)]
pub struct MempoolTransaction {
    txid: String,
    fee: u64,
    weight: u32,
    fee_rate: f64,
    parent_txids: Vec<String>
}

impl MempoolTransaction {

    fn fetch_mempool_txns() -> Result<Vec<MempoolTransaction>, Box<dyn Error>> {
        let mut mempool_txns: Vec<MempoolTransaction> = vec![];

        let raw_mempool: Vec<u8> = bcli(&format!("getrawmempool true")).expect("Error getting raw mempool");
        
        let mempool_data_str = String::from_utf8(raw_mempool).expect("Failed to convert bytes to string");
        let mempool_data: HashMap<String, TransactionData> = serde_json::from_str(&mempool_data_str).expect("Could not deserialize mempool data");

        for (txid, data) in mempool_data {
            let fee: u64 = 0;
            let weight = data.weight;
            let fee_rate: f64= fee as f64 / weight as f64;
            let parent_txids = vec![];

            mempool_txns.push(MempoolTransaction {
                txid,
                fee,
                weight,
                fee_rate,
                parent_txids
            })
        }

        Ok(mempool_txns)
    }

}

pub fn run() -> Result<Vec<MempoolTransaction>, Box<dyn Error>> {
    let mempool_txns: Vec<MempoolTransaction> = MempoolTransaction::fetch_mempool_txns()?;

    Ok(mempool_txns)
}
