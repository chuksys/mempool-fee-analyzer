pub mod block_template_median;
use crate::mempool_data::MempoolTransaction;
use crate::strategies::block_template_median::BlockTemplateMedianEstimator;

pub trait FeeRateEstimator {
    fn estimate_fee_rate(&self, mempool_data: &Vec<MempoolTransaction>) -> f64;
    fn name(&self) -> &'static str;
}

pub fn select_strategy(strategy_name: &str) -> Box<dyn FeeRateEstimator> {
    match strategy_name {
        "block_template_median" => Box::new(BlockTemplateMedianEstimator),
        _ => panic!("Unknown strategy!"),
    }
}
