use std::error::Error;
use crate::config::Config;
use crate::mempool_data::MempoolTransaction;
use crate::strategies::{FeeEstimator, select_strategy};

pub struct Runner<'a> {
    pub strategy: &'a dyn FeeEstimator,
}

impl<'a> Runner<'a> {
    pub fn new(strategy: &'a dyn FeeEstimator) -> Self {
        Runner { strategy }
    }

    pub fn run_analysis(&self, mempool_data: Vec<MempoolTransaction>) -> f64 {
        self.strategy.estimate_fee(mempool_data)
    }
}

pub fn run_strategy(config: Config) -> Result<(), Box<dyn Error>> {
    
    let strategy = select_strategy(&config.strategy_name);
    let runner = Runner::new(&*strategy);
    let mempool_data: Vec<MempoolTransaction> = MempoolTransaction::fetch_mempool_txns()?;

    let _ = runner.run_analysis(mempool_data);

    Ok(())
}