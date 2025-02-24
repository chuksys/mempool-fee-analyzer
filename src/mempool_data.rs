#![allow(unused)]
use std::{error::Error, os::unix::process::parent_id, path::PathBuf, process::Command};
use hex::FromHex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::Path;
use std::io;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct Fees {
    ancestor: f64,
    base: f64,
    descendant: f64,
    modified: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MempoolData {
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
    weight: u64,
    wtxid: String,
}

impl MempoolData {
    pub fn save_to_file(mempool_data: &HashMap<String, MempoolData>, file_path: &str) -> io::Result<()> {
        let serialized = serde_json::to_string(&mempool_data).unwrap();
        fs::write(file_path, serialized)
    }

    pub fn load_from_file(file_path: &str) -> io::Result<HashMap<String, MempoolData>> {
        let data = fs::read_to_string(file_path)?;
        let mempool_data = serde_json::from_str(&data).unwrap();
        Ok(mempool_data)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MempoolTransaction {
    pub txid: String,
    pub fee: u64,
    pub weight: u64,
    pub fee_rate: f64,
    pub parent_txids: Vec<String>,
    pub inputs_count: u32,
    pub outputs_count: u32
}

impl MempoolTransaction {

    pub fn fetch_mempool_txns(mempool_data: &HashMap<String, MempoolData>) -> Result<Vec<MempoolTransaction>, Box<dyn Error>> {
        let mut mempool_txns: Vec<MempoolTransaction> = vec![];

        for (txid, data) in mempool_data {
            let fee: u64 = (data.fees.base * 100_000_000.0) as u64;
            let weight = data.weight;
            let fee_rate: f64 = fee as f64 / weight as f64;
            let parent_txids = &data.depends;
            let inputs_count = 0;
            let outputs_count = 0;

            mempool_txns.push(MempoolTransaction {
                txid: txid.to_string(),
                fee,
                weight,
                fee_rate,
                parent_txids: parent_txids.to_vec(),
                inputs_count,
                outputs_count
            })
        }

        Ok(mempool_txns)
    }


}
