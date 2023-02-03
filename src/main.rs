pub mod utils;

use serde::Deserialize;
pub use utils::*;
use web3::types::H160;
use web3_rust_wrapper::Web3Manager;

#[derive(Debug, Deserialize)]
pub struct BotConfig {
    pub account_prk: String,
    pub router_address: String,
    pub token_address: String,
    pub invest_amount: f64,
    pub max_slipage: f64,
    pub stop_loss: f64,
    pub take_profit_percent: f64,
    pub ath_take_profit_percent: f64,
    pub network: String,
}

#[tokio::main]
async fn main() -> web3::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(2).unwrap();
    let file = std::fs::File::open(filename).unwrap();
    let config: BotConfig = serde_json::from_reader(file).unwrap();

    let mut price_history: Vec<(f32, f32)> = Vec::new();


    let network = match config.network.as_str() {
        "bsc" => web3_rust_wrapper::Network::BSCMainnet,
        "bsc-testnet" => web3_rust_wrapper::Network::BSCTestnet,
        "eth" => web3_rust_wrapper::Network::ETHMainnet,
        _ => web3_rust_wrapper::Network::BSCMainnet,
    };

    // INITIALIZE Web3Manager
    let mut web3m: Web3Manager = Web3Manager::new(network).await;

    // INITIALIZE ACCOUNT
    web3m.load_account(config.account_prk.as_str()).await;
    let account: H160 = web3m.first_account();

    // 1. CHECK IF TOKEN HAS LIQUIDITY
    // 2. CHECK TRADING ENABLE
    // 3. CALC BUY SELL FEES
    // 4. CHECK HONEYPOT
    check_before_buy(
        &mut web3m,
        account,
        config.router_address.as_str(),
        config.token_address.as_str(),
    )
    .await;

    // 4. DO REAL BUY
    let buy_price = do_real_buy(
        &mut web3m,
        account,
        config.router_address.as_str(),
        config.token_address.as_str(),
        eth_to_wei(config.invest_amount, 18),
    )
    .await;
    clear_screen();

    // 5. LOOP UNTIL TAKE PROFIT OR STOP LOSS
    do_real_sell(
        &mut web3m,
        account,
        config.token_address.as_str(),
        config.router_address.as_str(),
        config.take_profit_percent,
        config.stop_loss,
        buy_price,
        config.ath_take_profit_percent,
        &mut price_history
    )
    .await;

    Ok(())
}
