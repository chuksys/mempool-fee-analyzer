#![allow(unused)]
use std::error::Error;
use crate::{block_data, mempool_data};
use crate::strategies::FeeEstimator;

#[derive(Debug)]
pub struct BlockTemplateMedianEstimator;

impl FeeEstimator for BlockTemplateMedianEstimator {

    fn estimate_fee(&self, mempool_data: Vec<mempool_data::MempoolTransaction>) -> f64 {
        
        //build block template
        let block_template = block_data::BlockBuilder::build_block(mempool_data).expect("Could not get block template");

        let half_block_template_length = block_template.len() as f64 / 2.0;
        let half_block_template_length_is_whole_number = (half_block_template_length.fract() == 0.0) || half_block_template_length.is_nan();

        let mut median_fee_rate: f64 = 0.0;

        if half_block_template_length_is_whole_number {
            let median_index_1 = half_block_template_length.trunc() as usize;
            let median_index_2 = half_block_template_length.trunc() as usize + 1;

            median_fee_rate = (block_template[median_index_1].fee_rate + block_template[median_index_2].fee_rate) / 2.0;
        } else {
            let median_index = half_block_template_length.trunc() as usize + 1;
            median_fee_rate = block_template[median_index].fee_rate;
        }

        median_fee_rate
    }

    fn name(&self) -> &'static str {
        "Block Template Median Strategy"
    }
}