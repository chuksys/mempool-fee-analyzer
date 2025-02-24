use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(short, long, help = "Name of the fee estimation strategy to use", value_name = "STRATEGY_NAME")]
    pub strategy_name: String,

    #[clap(subcommand)]
    pub commands: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Comparison {
    LessThan,
    GreaterThan,
    Equals,
}

fn parse_comparison_and_count(s: &str) -> Result<(Comparison, u32), String> {
    let parts: Vec<&str> = s.split(' ').collect();
    if parts.len() != 2 {
        return Err("Invalid format. Use '<comparison> <count>' (e.g., less-than 5)".to_string());
    }

    let comparison = match parts[0].to_lowercase().as_str() {
        "less-than" => Ok(Comparison::LessThan),
        "greater-than" => Ok(Comparison::GreaterThan),
        "equals" => Ok(Comparison::Equals),
        _ => Err("Invalid comparison. Use less-than, greater-than, or equals.".to_string()),
    }?;

    let count = parts[1].parse::<u32>().map_err(|_| "Invalid count. Must be a positive integer.".to_string())?;

    Ok((comparison, count))
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    SubsetSample {
        #[clap(group = "fee_tier")]
        #[clap(long, help = "Select low-fee transactions.")]
        low_fee_txns: bool,

        #[clap(group = "fee_tier")]
        #[clap(long, help = "Select high-fee transactions.")]
        high_fee_txns: bool,

        #[clap(long, help = "Select transactions with inputs <comparison> <count>.", value_parser = parse_comparison_and_count)]
        inputs: Option<(Comparison, u32)>,

        #[clap(long, help = "Select transactions with outputs <comparison> <count>.", value_parser = parse_comparison_and_count)]
        outputs: Option<(Comparison, u32)>,
    }
}

pub fn parse_config() -> Config {
    Config::parse()
}