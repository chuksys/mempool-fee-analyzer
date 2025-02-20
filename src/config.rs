use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(short, long, help = "Name of the fee estimation strategy to use")]
    pub strategy_name: String,

    #[arg(short, long, help = "Subset of mempool data to analyze (e.g., 'low-fee-txns', 'high-fee-txns', 'less than 5 inputs', '5 outputs')")]
    pub mempool_subset: String,
}

pub fn parse_config() -> Config {
    Config::parse()
}