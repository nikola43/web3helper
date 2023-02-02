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
}

#[tokio::main]
async fn main() -> web3::Result<()> {
    dotenv::dotenv().ok();

    // GET FILENAME FROM ARGUMENTS
    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(2).unwrap();

    let file = std::fs::File::open(filename).unwrap();

    // LOAD CONFIG FROM JSON FILE
    let config: BotConfig = serde_json::from_reader(file).unwrap();

    // INITIALIZE Web3Manager
    let mut web3m: Web3Manager = Web3Manager::new(web3_rust_wrapper::Network::BSCTestnet).await;

    // INITIALIZE ACCOUNT
    web3m.load_account(config.account_prk.as_str()).await;
    let account: H160 = web3m.first_loaded_account();

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
    )
    .await;

    Ok(())
}
