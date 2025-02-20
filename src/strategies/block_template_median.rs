#![allow(unused)]
use std::error::Error;
use crate::mempool_data;
use crate::strategies::FeeEstimator;

#[derive(Debug)]
pub struct BlockTemplateMedianEstimator;

impl FeeEstimator for BlockTemplateMedianEstimator {
    
    fn estimate_fee(&self, mempool_data: Vec<mempool_data::MempoolTransaction>) -> f64 {
        
        println!("mempool_data {:?}", mempool_data.len());

        //build block template
        //extract 2 subsets from block template - txns above median and txns below median - so we can see how the estimation model performs for low-fee txns as well as high-fee txns.
        //save txids from these 2 subsets for evaluation later
        //find the median of each split
        //repeat process every minute and log medians in a csv

        //monitor target block confirmation
        //Upon confirmation, fetch all txns included in the target block which were part of subsets extracted earlier and group them accordingly
        //find the median for each of these groups
        //compare them with the medians from block template subsets logged in the csv. 

        1.0000000
    }

    fn name(&self) -> &'static str {
        "Block Template Median Strategy"
    }
}