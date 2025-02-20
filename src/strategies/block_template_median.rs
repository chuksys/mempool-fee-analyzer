use std::error::Error;
use crate::mempool_data;

pub fn run() -> Result<(), Box<dyn Error>> {
    let mempool_txns = mempool_data::run().expect("Error Fetching Mempool Data");
    println!("mempool_txns_count {:?}", mempool_txns.len());
    //build block template
    //extract 2 subsets from block template - txns above median and txns below median - so we can see how the estimation model performs for low-fee txns as well as high-fee txns.
    //save txids from these 2 subsets for evaluation later
    //find the median of each split
    //repeat process every minute and log medians in a csv

    //monitor target block confirmation
    //Upon confirmation, fetch all txns included in the target block which were part of subsets extracted earlier and group them accordingly
    //find the median for each of these groups
    //compare them with the medians from block template subsets logged in the csv. 

    Ok(()) 
}