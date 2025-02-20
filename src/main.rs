use mempool_fee_analyzer::{config::parse_config, runner::run_strategy};
fn main() {
    let config = parse_config();
    let _ = run_strategy(config);
}
