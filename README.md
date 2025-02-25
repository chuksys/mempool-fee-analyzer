# Mempool Fee Analyzer

This open-source project provides a comprehensive tool for evaluating and comparing the reliability of various mempool-based fee estimation strategies for Bitcoin transactions. Our goal is to empower users and developers with the insights needed to optimize transaction fees and improve the efficiency of the Bitcoin network.

## Why Fee Estimation Matters

Accurate fee estimation is crucial for a smooth and cost-effective Bitcoin experience. Overpaying for fees wastes resources, while underpaying can lead to significant delays in transaction confirmation. By analyzing mempool dynamics, we can develop and refine fee estimation strategies that minimize costs and ensure timely inclusion in blocks.

## Project Goals

This project aims to:

* **Evaluate Existing Strategies:** Systematically test and compare the performance of multiple mempool-based fee estimation algorithms under diverse conditions.
* **Provide Actionable Insights:** Offer clear and concise metrics on the reliability and effectiveness of each strategy, enabling users to make informed decisions about transaction fees.
* **Foster Community Collaboration:** Encourage community involvement in the development and improvement of fee estimation techniques.
* **Promote Transparency:** Increase transparency into mempool dynamics and the factors that influence transaction fees.

## Features

We are starting with a command-line interface (CLI) and plan to add a graphical user interface (GUI) in the future. Current and planned features include:

* **Real-time Bitcoin Fee Optimization Insights:** This project analyzes real-time Bitcoin transaction data to provide actionable insights into fee optimization. We compare actual transaction fees with hypothetical fees calculated using various mempool-based strategies, highlighting how users can reduce their transaction costs and improve confirmation times in real time.
* **Granular Analysis by Transaction Size:**  We will perform a granular analysis of fee estimation performance based on transaction size, considering the number of inputs and outputs.  This will allow us to identify strategies that are optimized for different types of transactions (e.g., simple transactions with few inputs/outputs vs. complex transactions with many inputs/outputs).
* **Data Visualization:**  Interactive charts and graphs to visualize fee trends, mempool dynamics, and the performance of different estimation strategies.
* **Customizable Parameters:**  Allow users to configure parameters for each strategy and tailor the analysis to their specific needs.
* **Real-time Mempool Monitoring:**  Integration with live mempool data sources for up-to-the-minute analysis.

## Prerequisites

Before using `mempool-fee-analyzer`, you need to have a running Bitcoin Core node. Make sure your Bitcoin Core node is properly configured and synchronized with the network.  You'll also need Rust and Cargo installed.  You can install Rust from [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).

## Getting Started

1. **Clone the Repository:**

```bash
git clone https://github.com/chuksys/mempool-fee-analyzer.git
cd mempool-fee-analyzer
```

2. **Run the Analyzer:**

The mempool-fee-analyzer tool is run using the cargo run command.  The general syntax is: 

```bash
cargo run -- --strategy-name <strategy_name> --duration <num_of_blocks> subset-sample [options]
```
`--strategy-name <strategy_name>`: Specifies the fee estimation strategy to use. We currently have one strategy included - 
**block_template_median**.

`--duration <num_of_blocks>`: Specifies the number of blocks to analyze.

`subset-sample`:  Indicates the subset sampling method.

`[options]`:  Additional options to filter mempool transactions for analysis.  You can choose one or more of the following:

* `--high-fee-txns`: Select high-fee transactions - to see how the strategy performs given high-fee txns.
* `--low-fee-txns`: Select low-fee transactions - to see how the strategy performs given low-fee txns.
* `--inputs '<comparison> <count>'`: Select transactions with a specific number of inputs.`<comparison>` can be equals, less-than or greater-than. `<count>` is the number of inputs to compare against.
* `--outputs '<comparison> <count>'`: Select transactions with a specific number of outputs (same comparison options as `--inputs`).

**Example Usage**

```bash
cargo run -- --strategy-name block_template_median --duration 100 subset-sample --high-fee-txns --inputs 'equals 2'
```

This command will:

* Use the `block_template_median` fee estimation strategy.
* Analyze the next `100` blocks.
* Select `high-fee` transactions mempool subset.
* Select transactions with `2` inputs.

```bash
cargo run -- --strategy-name block_template_median --duration 50 subset-sample --low-fee-txns --outputs 'greater-than 2'
```

This command will:

* Use the `block_template_median` fee estimation strategy.
* Analyze the next `50` blocks.
* Select `low-fee` transactions mempool subset.
* Select transactions with more than `2` outputs.


## Contributing

We welcome contributions from the community!  Please see our [Contribution Guide](CONTRIBUTING.md) for details on how to get involved. We encourage contributions of all kinds, including:

* New mempool-based fee estimation strategies
* Performance improvements
* Bug fixes
* Documentation enhancements
* GUI development

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contact

Join our [discord server](https://discord.gg/3Tfs6eAfe7) to be a part of the community of people deliberating better ways to advance mempool-based fee estimation strategies.

## Acknowledgements

This project is highly inspired by Abubakar Sadiq Ismail's work on [mempool-fee based estimation](https://delvingbitcoin.org/t/mempool-based-fee-estimation-on-bitcoin-core/703).