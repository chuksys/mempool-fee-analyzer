use crate::mempool_data::MempoolTransaction;
use crate::config::Comparison;

pub trait MempoolTransactionFilter {
    fn filter(&self, txn: &MempoolTransaction) -> bool;
}

pub struct LowFeeFilter {
    pub threshold: f64,
}

impl MempoolTransactionFilter for LowFeeFilter {
    fn filter(&self, txn: &MempoolTransaction) -> bool {
        txn.fee_rate < self.threshold
    }
}

pub struct HighFeeFilter {
    pub threshold: f64,
}

impl MempoolTransactionFilter for HighFeeFilter {
    fn filter(&self, txn: &MempoolTransaction) -> bool {
        txn.fee_rate >= self.threshold
    }
}

pub struct InputsCountFilter {
    pub comparison: Comparison,
    pub count: u32,
}

impl MempoolTransactionFilter for InputsCountFilter {
    fn filter(&self, txn: &MempoolTransaction) -> bool {
        match self.comparison {
            Comparison::LessThan => txn.inputs_count < self.count,
            Comparison::GreaterThan => txn.inputs_count > self.count,
            Comparison::Equals => txn.inputs_count == self.count,
        }
    }
}

pub struct OutputsCountFilter {
    pub comparison: Comparison,
    pub count: u32,
}

impl MempoolTransactionFilter for OutputsCountFilter {
    fn filter(&self, txn: &MempoolTransaction) -> bool {
        match self.comparison {
            Comparison::LessThan => txn.outputs_count < self.count,
            Comparison::GreaterThan => txn.outputs_count > self.count,
            Comparison::Equals => txn.outputs_count == self.count,
        }
    }
}

pub fn filter_mempool_txns(txns: &[MempoolTransaction], filters: &[Box<dyn MempoolTransactionFilter>]) -> Vec<MempoolTransaction> {
    txns.iter()
    .filter(|txn| filters.iter().all(|f| f.filter(txn)))
    .cloned()
    .collect()
}