use mempool_fee_analyzer::{config::parse_config, runner::run_tasks};

#[tokio::main]
async fn main() {
    let config = parse_config();
    let _ = run_tasks(config).await;
}
