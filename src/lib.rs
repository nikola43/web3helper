extern crate alloc;

pub mod ethereum_mainnet;
pub mod rinkeby_testnet;
pub mod traits;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use ethereum_private_key_to_address::PrivateKey;
use futures::StreamExt;
use secp256k1::rand::rngs::StdRng;
use secp256k1::rand::Rng;
use secp256k1::{
    rand::{rngs, SeedableRng},
    PublicKey, SecretKey,
};
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::convert::{From, TryFrom};
use std::error::Error;
use std::str::FromStr;
use std::time::SystemTime;
use web3::api::SubscriptionStream;
use web3::contract::tokens::{Detokenize, Tokenize};
use web3::contract::{Contract, Options};
use web3::ethabi::ethereum_types::H256;
use web3::ethabi::{Int, Uint};
use web3::helpers as w3h;
use web3::signing::keccak256;
use web3::transports::{Http, WebSocket};
use web3::types::{
    Address, BlockNumber, Bytes, FilterBuilder, Log, SignedTransaction, TransactionId,
    TransactionParameters, H160, U256, U64,
};
use web3::Web3;

// use hex_literal::hex;

/// Emulates a `switch` statement.
///
/// The syntax is similar to `match` except that every left-side expression is
/// interpreted as an expression rather than a pattern. The expression to
/// compare against must be at the beginning with a semicolon. A default case
/// is required at the end with a `_`, similar to `match`.
///
/// Example:
///
/// ```
/// use switch_statement::switch;
/// use web3_rust_wrapper::switch;
///
/// const A: u32 = 1 << 0;
/// const B: u32 = 1 << 1;
///
/// let n = 3;
/// let val = switch! { n;
///     A => false,
///     // this is a bitwise OR
///     A | B => true,
///     _ => false,
/// };
/// assert!(val);
/// ```
#[macro_export]
macro_rules! switch {
    ($v:expr; $($a:expr => $b:expr,)* _ => $e:expr $(,)?) => {
        match $v {
            $(v if v == $a => $b,)*
            _ => $e,
        }
    };
}

#[cfg(test)]
mod tests {
    const A: u32 = 1 << 0;
    const B: u32 = 1 << 1;
    const C: u32 = 1 << 2;
    const D: u32 = 1 << 3;

    #[test]
    fn it_works() {
        assert!(switch! { 1; _ => true });

        let v = switch! { A | B;
            A => false,
            B | C => false,
            A | B => true,
            C | D => {
                unreachable!();
            },
            _ => false,
        };
        assert!(v);
    }

    #[test]
    fn no_trailing_comma() {
        let v = switch! { 1;
            1 => true,
            _ => false
        };
        assert!(v);
    }
}

// use chainlink_interface::EthereumFeeds;
trait InstanceOf
where
    Self: Any,
{
    fn instance_of<U: ?Sized + Any>(&self) -> bool {
        TypeId::of::<Self>() == TypeId::of::<U>()
    }
}

// implement this trait for every type that implements `Any` (which is most types)
impl<T: ?Sized + Any> InstanceOf for T {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyPair {
    pub secret_key: String,
    pub public_key: String,
}

#[derive(Clone, Debug)]
pub struct EVMNetwork {
    pub http_url: String,
    pub ws_url: String,
    pub chain_id: Option<u64>,
}

pub enum Network {
    ETHMainnet = 1,
    ETHGoerli = 5,
    ETHSepolia = 11155111,
    BSCMainnet = 56,
    BSCTestnet = 97,
    AvalancheMainnet = 99,
    AvalancheTestnet = 100,
}

impl EVMNetwork {
    pub fn new(network_id: Network) -> EVMNetwork {
        let mut _http_url = "";
        let mut _socket_url = "";

        match network_id {
            Network::ETHMainnet => {
                _http_url = "https://rpc.ankr.com/eth";
                _socket_url = "wss://mainnet.infura.io/ws/v3/3bbe50d0eb1c4462ad6823ea8aa216f8";
            }
            Network::ETHGoerli => {
                _http_url = "https://goerli.infura.io/v3/d39a866f4f6d49b9916f9269bf880110";
                _socket_url = "wss://goerli.infura.io/ws/v3/d39a866f4f6d49b9916f9269bf880110";
            }
            Network::ETHSepolia => {
                _http_url = "https://sepolia.infura.io/v3/d39a866f4f6d49b9916f9269bf880110";
                _socket_url = "wss://sepolia.infura.io/ws/v3/d39a866f4f6d49b9916f9269bf880110";
            }
            Network::BSCMainnet => {
                _http_url = "https://rpc.ankr.com/bsc";
                _socket_url =
                    "wss://bsc-mainnet.nodereal.io/ws/v1/d4224d2458594df5830eb45cdef8b45b";
            }
            Network::BSCTestnet => {
                //_http_url = "https://rpc.ankr.com/bsc_testnet_chapel";
                //_socket_url = "wss://bsc-testnet.nodereal.io/ws/v1/d4224d2458594df5830eb45cdef8b45b";

                _http_url = "https://rpc.ankr.com/bsc_testnet_chapel";
                _socket_url =
                    "wss://bsc-testnet.nodereal.io/ws/v1/d4224d2458594df5830eb45cdef8b45b";
            }
            Network::AvalancheMainnet => {
                _http_url = "https://speedy-nodes-nyc.moralis.io/84a2745d907034e6d388f8d6/avalanche/mainnet";
                _socket_url = "wss://speedy-nodes-nyc.moralis.io/84a2745d907034e6d388f8d6/avalanche/mainnet/ws";
            }
            Network::AvalancheTestnet => {
                _http_url = "https://speedy-nodes-nyc.moralis.io/84a2745d907034e6d388f8d6/avalanche/mainnet";
                _socket_url = "wss://speedy-nodes-nyc.moralis.io/84a2745d907034e6d388f8d6/avalanche/mainnet/ws";
            }
            _ => {
                _http_url = "https://speedy-nodes-nyc.moralis.io/84a2745d907034e6d388f8d6/avalanche/mainnet";
                _socket_url = "wss://speedy-nodes-nyc.moralis.io/84a2745d907034e6d388f8d6/avalanche/mainnet/ws";
            }
        }

        let u64chain_id: u64 = network_id as u64;
        println!("chain id: {}", u64chain_id);
        let chain_id: Option<u64> = Option::Some(u64::try_from(u64chain_id).unwrap());

        EVMNetwork {
            http_url: String::from(_http_url),
            ws_url: String::from(_socket_url),
            chain_id,
        }
    }
}

#[test]
fn new_network() {
    let network = EVMNetwork::new(Network::BSCMainnet);
    println!("{:?}", network);
}

// 0x9Ac64Cc6e4415144C455BD8E4837Fea55603e5c3
#[derive(Clone, Debug)]
pub struct Router {
    pub address: String,
    pub factory: String,
}

impl Router {
    pub async fn new(network_id: Network) {}
}

#[derive(Clone, Debug)]
pub struct Web3Manager {
    // all the accounts
    pub accounts: Vec<H160>,
    // public addresses
    pub web3http: Web3<Http>,
    // web3 https instance (for use call or write contract functions)
    pub web3web_socket: Web3<WebSocket>,
    // web3 websocket instance (for listen contracts events)
    accounts_map: HashMap<H160, String>,
    current_nonce: U256,
    // hashmap (like mapping on solidity) for store public and private keys
    chain_id: Option<u64>,
}

impl Web3Manager {
    //-------------------------------------------------------------------------
    //                        getters
    //-------------------------------------------------------------------------
    pub fn get_current_nonce(&self) -> U256 {
        self.current_nonce
    }

    //-------------------------------------------------------------------------
    //                        setters
    //-------------------------------------------------------------------------
    pub fn set_current_nonce(&mut self, new_nonce: U256) {
        self.current_nonce = new_nonce;
    }

    /*
    pub async fn call_write_function<P: Clone>(
        &mut self,
        account: H160,
        contract_instance: Contract<Http>,
        contract_function: String,
        params: P,
    ) -> Result<H256, web3::Error>
    where
        P: Tokenize,
    {
        let tx_result = self
            .sign_and_send_tx(
                account,
                &contract_instance,
                &contract_function.to_string(),
                &params,
                U256::from_dec_str("0").unwrap(),
            )
            .await;
        tx_result
    }
    */

    pub async fn instance_contract(
        &self,
        plain_contract_address: &str,
        abi_path: &[u8],
    ) -> Result<Contract<Http>, Box<dyn std::error::Error>> {
        Ok(Contract::from_json(
            self.web3http.eth(),
            Address::from_str(plain_contract_address)?,
            abi_path,
        )?)
    }

    pub fn generate_keypair() -> (SecretKey, PublicKey) {
        let secp = secp256k1::Secp256k1::new();

        let n2: u64 = 1;
        println!("first random u64 is: {}", n2);
        let mut rng: StdRng = rngs::StdRng::seed_from_u64(n2);
        let random_number: u64 = rng.gen::<u64>();
        println!(
            "With seed {}, the first random u64 is: {}",
            n2, random_number
        );

        secp.generate_keypair(&mut rng)
    }

    pub fn public_key_address(public_key: &PublicKey) -> Address {
        let public_key = public_key.serialize_uncompressed();
        debug_assert_eq!(public_key[0], 0x04);
        let hash = keccak256(&public_key[1..]);
        Address::from_slice(&hash[12..])
    }

    pub fn generate_keypairs(n: u8) -> Vec<(SecretKey, PublicKey)> {
        let mut keypairs: Vec<(SecretKey, PublicKey)> = Vec::new();
        for _ in 0..n {
            keypairs.push(Web3Manager::generate_keypair());
        }
        keypairs
    }

    pub async fn get_token_balance(&self, token_address: &str, account: H160) -> U256 {
        let token_abi = include_bytes!("../abi/TokenAbi.json");
        let token_instance: Contract<Http> = self
            .instance_contract(token_address, token_abi)
            .await
            .unwrap();

        /*
        let token_decimals: U256 = self
            .query_contract(&token_instance, "decimals", ())
            .await
            .unwrap();
            */

        let token_balance: U256 = self
            .query_contract(&token_instance, "balanceOf", account)
            .await
            .unwrap();

        token_balance
    }

    pub fn generate_deadline(&self) -> U256 {
        U256::from(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        ) + 10000usize
    }

    // TODO(elsuizo:2022-03-03): documentation here
    pub async fn swap_tokens_for_exact_tokens(
        &mut self,
        account: H160,
        router_address: &str,
        token_amount: U256,
        pairs: &[&str],
        slippage: usize,
    ) -> Result<H256, Box<dyn Error>> {
        let contract_function = "swapTokensForExactTokens";

        let router_abi = include_bytes!("../abi/PancakeRouterAbi.json");
        let router_instance: Contract<Http> = self
            .instance_contract(router_address, router_abi)
            .await
            .expect("error creating the router instance");

        let mut addresses = Vec::new();
        for pair in pairs {
            addresses.push(Address::from_str(pair).unwrap());
        }

        let parameter_out = (token_amount, addresses.clone());
        let amount_out_min: Vec<Uint> = self
            .query_contract(&router_instance, "getAmountsOut", parameter_out)
            .await
            .unwrap();

        let min_amount = U256::from(amount_out_min[1].as_u128());
        let min_amount_less_slippage = min_amount - ((min_amount * slippage) / 100usize);

        let parameters2 = (
            token_amount,
            min_amount_less_slippage,
            addresses,
            self.first_account(),
            self.generate_deadline(),
        );

        println!("amount_out: {:?}", token_amount);
        println!("min_amount_less_slippage: {:?}", min_amount_less_slippage);

        let send_tx_result = self
            .sign_and_send_tx(
                account,
                &router_instance,
                contract_function,
                &parameters2,
                token_amount,
            )
            .await;

        send_tx_result
    }

    pub async fn get_token_allowance(
        &self,
        token_address: &str,
        account: H160,
        spender: H160,
    ) -> U256 {
        let token_abi = include_bytes!("../abi/TokenAbi.json");
        let token_instance: Contract<Http> = self
            .instance_contract(token_address, token_abi)
            .await
            .unwrap();

        let token_allowance: U256 = self
            .query_contract(&token_instance, "allowance", (account, spender))
            .await
            .unwrap();

        token_allowance
    }

    pub async fn get_token_price(&mut self, router_address: &str, token_address: &str) -> U256 {
        let router_abi = include_bytes!("../abi/PancakeRouterAbi.json");
        let router_instance: Contract<Http> = self
            .instance_contract(router_address, router_abi)
            .await
            .expect("error creating the router instance");
        let weth_address = self.get_weth_address(&router_instance).await;

        let amount_out: U256 = U256::from_dec_str("1000000000000000000").unwrap();

        let mut addresses: Vec<H160> = Vec::new();
        addresses.push(Address::from_str(token_address).unwrap());
        addresses.push(Address::from_str(weth_address.as_str()).unwrap());

        let parameter_out = (amount_out, addresses);
        let amount_out_min: Vec<Uint> = self
            .query_contract(&router_instance, "getAmountsOut", parameter_out)
            .await
            .unwrap();
        let min_amount = U256::from(amount_out_min[1].as_u128());

        min_amount
    }

    pub async fn swap_exact_tokens_for_tokens_supporting_fee_on_transfer_tokens(
        &mut self,
        account: H160,
        router_address: &str,
        token_amount: U256,
        pairs: &[&str],
    ) -> Result<H256, Box<dyn Error>> {
        let contract_function: &str = "swapExactTokensForTokensSupportingFeeOnTransferTokens";

        let router_abi = include_bytes!("../abi/PancakeRouterAbi.json");
        let router_instance: Contract<Http> = self
            .instance_contract(router_address, router_abi)
            .await
            .expect("error creating the router instance");

        let mut addresses = Vec::new();
        for pair in pairs {
            addresses.push(Address::from_str(pair).unwrap());
        }

        let parameters = (
            token_amount,
            U256::from_dec_str("0").unwrap(),
            addresses,
            account,
            self.generate_deadline(),
        );

        let send_tx_result = self
            .sign_and_send_tx(
                account,
                &router_instance,
                contract_function,
                &parameters,
                U256::from_dec_str("0").unwrap(),
            )
            .await;

        send_tx_result
    }

    pub async fn swap_exact_tokens_for_eth(
        &mut self,
        account: H160,
        router_address: &str,
        token_address: &str,
        token_amount: U256,
        slippage: usize,
    ) -> Result<H256, Box<dyn Error>> {
        let contract_function: &str = "swapExactTokensForETH";

        let router_abi = include_bytes!("../abi/PancakeRouterAbi.json");
        let router_instance: Contract<Http> = self
            .instance_contract(router_address, router_abi)
            .await
            .expect("error creating the router instance");
        let weth_address = self.get_weth_address(&router_instance).await;

        let mut addresses: Vec<H160> = Vec::new();
        addresses.push(Address::from_str(token_address).unwrap());
        addresses.push(Address::from_str(weth_address.as_str()).unwrap());

        let parameter_out = (token_amount, addresses.clone());
        let amount_out_min: Vec<Uint> = self
            .query_contract(&router_instance, "getAmountsOut", parameter_out)
            .await
            .unwrap();

        let min_amount = U256::from(amount_out_min[1].as_u128());
        let min_amount_less_slippage = min_amount - ((min_amount * slippage) / 100);

        let parameters = (
            token_amount,
            min_amount_less_slippage,
            addresses,
            account,
            self.generate_deadline(),
        );

        println!("token_amount: {:?}", token_amount);
        println!("min_amount_less_slippage: {:?}", min_amount_less_slippage);
        println!("amount_out_min: {:?}", amount_out_min);

        let send_tx_result = self
            .sign_and_send_tx(
                account,
                &router_instance,
                contract_function,
                &parameters,
                U256::from_dec_str("0").unwrap(),
            )
            .await;

        send_tx_result
    }

    pub async fn swap_exact_tokens_for_eth_supporting_fee_on_transfer_tokens(
        &mut self,
        account: H160,
        router_address: &str,
        token_address: &str,
        token_amount: U256,
        slippage: usize,
    ) -> Result<H256, Box<dyn Error>> {
        let contract_function: &str = "swapExactTokensForETHSupportingFeeOnTransferTokens";

        let router_abi = include_bytes!("../abi/PancakeRouterAbi.json");
        let router_instance: Contract<Http> = self
            .instance_contract(router_address, router_abi)
            .await
            .expect("error creating the router instance");
        let weth_address = self.get_weth_address(&router_instance).await;

        let mut addresses: Vec<H160> = Vec::new();
        addresses.push(Address::from_str(token_address).unwrap());
        addresses.push(Address::from_str(weth_address.as_str()).unwrap());

        let parameter_out = (token_amount, addresses.clone());
        let amount_out_min: Vec<Uint> = self
            .query_contract(&router_instance, "getAmountsOut", parameter_out)
            .await
            .unwrap();

        let min_amount = U256::from(amount_out_min[1].as_u128());
        let min_amount_less_slippage = min_amount - ((min_amount * slippage) / 100);

        let parameters = (
            token_amount,
            min_amount_less_slippage,
            addresses,
            account,
            self.generate_deadline(),
        );

        println!("token_amount: {:?}", token_amount);
        println!("min_amount_less_slippage: {:?}", min_amount_less_slippage);
        println!("amount_out_min: {:?}", amount_out_min);

        let send_tx_result = self
            .sign_and_send_tx(
                account,
                &router_instance,
                contract_function,
                &parameters,
                U256::from_dec_str("0").unwrap(),
            )
            .await;

        send_tx_result
    }

    pub async fn swap_eth_for_exact_tokens(
        &mut self,
        account: H160,
        router_address: &str,
        token_address: &str,
        eth_amount: U256,
        slippage: usize,
    ) -> Result<H256, Box<dyn Error>> {
        let mut router_abi_path = "../abi/PancakeRouterAbi.json";
        let mut contract_function: &str = "swapExactETHForTokens";

        let router_abi = include_bytes!("../abi/PancakeRouterAbi.json");
        let router_instance: Contract<Http> = self
            .instance_contract(router_address, router_abi)
            .await
            .unwrap();

        let weth_address = self.get_weth_address(&router_instance).await;

        let mut path_address: Vec<&str> = vec![];

        switch! { router_address;
            "0xD99D1c33F9fC3444f8101754aBC46c52416550D1" => {

                router_abi_path = "../abi/PancakeRouterAbi.json";
                contract_function = "swapExactETHForTokens";

                path_address.push(weth_address.as_str());
                path_address.push(token_address);

            },
                "0x10ed43c718714eb63d5aa57b78b54704e256024e" => {

                router_abi_path = "../abi/PancakeRouterAbi.json";
                contract_function = "swapExactETHForTokens";

                path_address.push(weth_address.as_str());
                path_address.push(token_address);

            },
            _ => {

                router_abi_path = "../abi/PancakeRouterAbi.json";
                contract_function = "swapExactETHForTokens";

                path_address.push(weth_address.as_str());
                path_address.push(token_address);
            },
        }

        let mut addresses = Vec::new();
        for pair in path_address {
            addresses.push(Address::from_str(pair).unwrap());
        }

        let parameter_out = (eth_amount, addresses.clone());
        let amount_out_min: Vec<Uint> = self
            .query_contract(&router_instance, "getAmountsOut", parameter_out)
            .await
            .unwrap();

        let min_amount = U256::from(amount_out_min[1].as_u128());
        let min_amount_less_slippage = min_amount - ((min_amount * slippage) / 100usize);

        let parameters = (
            min_amount_less_slippage,
            addresses,
            account,
            self.generate_deadline(),
        );

        /*
        println!("slippage {}", slippage);
        println!("eth_amount {}", eth_amount);
        println!("amount_out_min[0] {}", amount_out_min[0]);
        println!("amount_out_min[1] {}", amount_out_min[1]);
        println!("min_amount_less_slippage {}", min_amount_less_slippage);
        */

        let send_tx_result = self
            .sign_and_send_tx(
                account,
                &router_instance,
                contract_function,
                &parameters,
                eth_amount,
            )
            .await;

        send_tx_result
    }

    pub async fn get_out_estimated_tokens_for_tokens(
        &mut self,
        contract_instance: &Contract<Http>,
        pair_a: &str,
        pair_b: &str,
        amount: &str,
    ) -> Result<U256, web3::contract::Error> {
        self.query_contract(
            contract_instance,
            "getAmountsOut",
            (
                amount.to_string(),
                vec![pair_a.to_string(), pair_b.to_string()],
            ),
        )
        .await
    }

    pub async fn get_eth_balance(&mut self, account: H160) -> U256 {
        let balance = self.web3http.eth().balance(account, None).await.unwrap();
        return balance;
    }

    // Counts the number of exececuted transactions by the loaded wallet to set the 'nonce' param for current transacction
    // Cuenta el número de transacciones se han ejecutado con la wallet cargada para establecer el parámetro 'nonce' en la transacción actual
    pub async fn last_nonce(&self, account: H160) -> U256 {
        let block_number: Option<BlockNumber> = Option::Some(BlockNumber::Pending);

        let nonce = self
            .web3http
            .eth()
            .transaction_count(account, block_number)
            .await
            .unwrap();

        return nonce;

        /*
        self.web3http
        .eth()
        .transaction_count(self.first_account(), None)
        .await
        */
    }

    pub async fn load_account(&mut self, plain_private_key: &str) {
        let private_key = PrivateKey::from_str(plain_private_key).unwrap();
        let wallet: H160 = H160::from_str(private_key.address().as_str()).unwrap();

        // push on account list
        self.accounts_map
            .insert(wallet, plain_private_key.to_string());
        self.accounts.push(wallet);

        // get last nonce from loaded account
        let nonce: U256 = self.last_nonce(wallet).await;

        self.set_current_nonce(nonce);
    }

    pub async fn new_from_rpc_url(
        http_url: &str,
        websocket_url: &str,
        u64chain_id: u64,
    ) -> Web3Manager {
        // init web3 http connection
        let web3http: Web3<Http> = web3::Web3::new(web3::transports::Http::new(http_url).unwrap());

        // init web3 ws connection
        let web3web_socket: Web3<WebSocket> = web3::Web3::new(
            web3::transports::WebSocket::new(websocket_url)
                .await
                .unwrap(),
        );

        // create empty vector for store accounts
        let accounts: Vec<Address> = vec![];
        let accounts_map: HashMap<H160, String> = HashMap::new();
        let current_nonce: U256 = U256::from_dec_str("0").unwrap();

        //let chain_id: Option<u64> = Option::Some(u64::try_from(web3http.eth().chain_id().await.unwrap()).unwrap());
        let chain_id: Option<u64> = Option::Some(u64::try_from(u64chain_id).unwrap());

        Web3Manager {
            accounts,
            web3http,
            web3web_socket,
            accounts_map,
            current_nonce,
            chain_id,
        }
    }

    pub async fn new(network_id: Network) -> Web3Manager {
        // http_url: &str, websocket_url: &str, u64chain_id: u64
        let network = EVMNetwork::new(network_id);

        // init web3 http connection
        let web3http: Web3<Http> =
            web3::Web3::new(web3::transports::Http::new(network.http_url.as_str()).unwrap());

        // init web3 ws connection
        println!("websocket url {}", network.ws_url.as_str());
        let web3web_socket: Web3<WebSocket> = web3::Web3::new(
            web3::transports::WebSocket::new(network.ws_url.as_str())
                .await
                .unwrap(),
        );

        // create empty vector for store accounts
        let accounts: Vec<Address> = vec![];
        let accounts_map: HashMap<H160, String> = HashMap::new();
        let current_nonce: U256 = U256::from_dec_str("0").unwrap();
        let chain_id: Option<u64> = Option::Some(u64::try_from(network.chain_id.unwrap()).unwrap());

        Web3Manager {
            accounts,
            web3http,
            web3web_socket,
            accounts_map,
            current_nonce,
            chain_id,
        }
    }

    // Get a estimation on medium gas price in network
    // Obtiene un precio del gas  estimado en la red
    pub async fn gas_price(&self) -> Result<U256, web3::Error> {
        self.web3http.eth().gas_price().await
    }

    // Get the current block in the network
    // Obtiene el número del bloque actual en la red
    pub async fn get_block(&self) -> Result<U64, web3::Error> {
        self.web3http.eth().block_number().await
    }

    pub async fn query_contract<P, T>(
        &self,
        contract_instance: &Contract<Http>,
        func: &str,
        params: P,
    ) -> Result<T, web3::contract::Error>
    where
        P: Tokenize,
        T: Detokenize,
    {
        // query contract
        contract_instance
            .query(func, params, None, Default::default(), None)
            .await
    }

    // The transactions must be signed with the private key of the wallet that executes it
    // Las transacciones han de ser firmadas con la clave privada de la cartera que la ejecuta
    pub async fn sign_transaction(
        &mut self,
        account: H160,
        transact_obj: TransactionParameters,
    ) -> SignedTransaction {
        let plain_pk = self.accounts_map.get(&account).unwrap();
        let private_key = SecretKey::from_str(plain_pk).unwrap();

        self.web3http
            .accounts()
            .sign_transaction(transact_obj, &private_key)
            .await
            .unwrap()
    }

    pub fn encode_tx_parameters(
        &mut self,
        nonce: U256,
        to: Address,
        value: U256,
        gas: U256,
        gas_price: U256,
        data: Bytes,
    ) -> TransactionParameters {
        TransactionParameters {
            nonce: Some(nonce),
            to: Some(to),
            value,
            gas_price: Some(gas_price),
            gas,
            data,
            chain_id: self.chain_id,
            ..Default::default()
        }
    }

    // TODO(elsuizo:2022-03-03): add a `Result` here
    pub fn encode_tx_data<P>(&mut self, contract: &Contract<Http>, func: &str, params: P) -> Bytes
    where
        P: Tokenize,
    {
        contract
            .abi()
            .function(func)
            .unwrap()
            .encode_input(&params.into_tokens())
            .unwrap()
            .into()
    }

    pub async fn estimate_tx_gasV1<P>(
        &mut self,
        contract: &Contract<Http>,
        func: &str,
        params: P,
        value: &str,
    ) -> U256
    where
        P: Tokenize,
    {
        let mut gas_estimation = U256::from_dec_str("0").unwrap();
        let gas_estimation_result = contract
            .estimate_gas(
                func,
                params,
                self.accounts[0],
                Options {
                    value: Some(U256::from_dec_str(value).unwrap()),
                    ..Default::default()
                },
            )
            .await;
        if gas_estimation_result.is_ok() {
            gas_estimation = gas_estimation_result.unwrap();
        }
        gas_estimation
    }

    pub fn first_account(&self) -> H160 {
        self.accounts[0]
    }

    pub async fn approve_erc20_token(
        &mut self,
        account: H160,
        token_address: &str,
        spender: &str,
        value: &str,
    ) -> Result<H256, web3::Error> {
        let token_abi = include_bytes!("../abi/TokenAbi.json");
        let token_instance: Contract<Http> = self
            .instance_contract(token_address, token_abi)
            .await
            .unwrap();

        let spender_address: Address = Address::from_str(spender).unwrap();
        let contract_function = "approve";
        let contract_function_parameters = (spender_address, U256::from_dec_str(value).unwrap());

        let send_tx_result = self
            .sign_and_send_tx(
                account,
                &token_instance,
                &contract_function.to_string(),
                &contract_function_parameters,
                U256::from_dec_str("0").unwrap(),
            )
            .await;

        Ok(send_tx_result.unwrap())
    }

    pub async fn sign_and_send_tx<P: Clone>(
        &mut self,
        account: H160,
        contract_instance: &Contract<Http>,
        func: &str,
        params: &P,
        value: U256,
    ) -> Result<H256, Box<dyn Error>>
    where
        P: Tokenize,
    {
        // estimate gas for call this function with this parameters
        // increase 200ms execution time, we use high gas available
        // gas not used goes back to contract
        //let estimated_tx_gas: U256 = U256::from_dec_str("5000000").unwrap();
        let gas_estimation_result = contract_instance
            .estimate_gas(
                func,
                params.clone(),
                account,
                Options {
                    value: Some(value),
                    ..Default::default()
                },
            )
            .await;

        if gas_estimation_result.is_err() {
            return Err(Box::new(gas_estimation_result.err().unwrap()));
        }
        let mut estimated_tx_gas = gas_estimation_result.unwrap();

        // increase estimated_tx_gas in 20%
        estimated_tx_gas += estimated_tx_gas / 5;

        // 2. encode_tx_data
        let tx_data: Bytes = self.encode_tx_data(contract_instance, func, params.clone());
        let gas_price: U256 = self.web3http.eth().gas_price().await.unwrap();
        let nonce: U256 = self.get_current_nonce();

        // 3. build tx parameters
        let tx_parameters: TransactionParameters = self.encode_tx_parameters(
            nonce,
            contract_instance.address(),
            value,
            estimated_tx_gas,
            gas_price,
            tx_data,
        );

        // 4. sign tx and send tx
        let tx_result = self.sign_and_send_transaction(account, tx_parameters).await;

        self.update_nonce();

        return Ok(tx_result.unwrap());
    }

    async fn sign_and_send_transaction(
        &mut self,
        account: H160,
        tx_parameters: TransactionParameters,
    ) -> Result<H256, web3::Error> {
        let signed_transaction: SignedTransaction =
            self.sign_transaction(account, tx_parameters).await;

        // send tx
        let tx_result = self
            .web3http
            .eth()
            .send_raw_transaction(signed_transaction.raw_transaction)
            .await;
        tx_result
    }

    fn update_nonce(&mut self) {
        self.set_current_nonce(self.get_current_nonce() + 1)
    }

    pub async fn sent_eth(&mut self, account: H160, to: H160, amount: &str) {
        let amount_out: U256 = U256::from_dec_str(amount).unwrap();

        // Build the tx object
        let tx_object = TransactionParameters {
            to: Some(to),
            value: amount_out, //0.1 eth
            ..Default::default()
        };

        let plain_pk = self.accounts_map.get(&account).unwrap();
        let private_key = SecretKey::from_str(plain_pk).unwrap();

        // Sign the tx (can be done offline)
        let signed = self
            .web3http
            .accounts()
            .sign_transaction(tx_object, &private_key)
            .await
            .unwrap();

        // Send the tx to infura
        let result = self
            .web3http
            .eth()
            .send_raw_transaction(signed.raw_transaction)
            .await
            .unwrap();

        println!("Tx succeeded with hash: {}", result);
    }

    pub async fn sent_erc20_token(
        &mut self,
        account: H160,
        contract_instance: Contract<Http>,
        to: &str,
        token_amount: &str,
    ) -> H256 {
        let contract_function = "transfer";

        let recipient_address: Address = Address::from_str(to).unwrap();
        let contract_function_parameters =
            (recipient_address, U256::from_dec_str(token_amount).unwrap());

        let send_tx_result = self
            .sign_and_send_tx(
                account,
                &contract_instance,
                contract_function,
                &contract_function_parameters,
                U256::from_dec_str("0").unwrap(),
            )
            .await;

        send_tx_result.unwrap()
    }

    //-------------------------------------------------------------------------
    //                        chainlink inplementations
    //-------------------------------------------------------------------------

    pub async fn get_latest_price(
        &mut self,
        network: impl crate::traits::GetAddress,
        pair_address: &str,
    ) -> Int {
        let proxy_abi = include_bytes!("../abi/EACAggregatorProxy.json");
        let proxy_instance: Contract<Http> = self
            .instance_contract(&network.get_address(pair_address).unwrap(), proxy_abi)
            .await
            .unwrap();

        let res: (Uint, Int, Uint, Uint, Uint) = self
            .query_contract(&proxy_instance, "latestRoundData", ())
            .await
            .unwrap();
        res.1
    }

    /*
        pub async fn access_controller(
        &mut self,
        feed: impl crate::traits::GetAddress,
        pair: &str,
    ) -> Result<Address, web3::contract::Error> {
        let proxy_abi = include_bytes!("../abi/EACAggregatorProxy.json");
        let proxy_instance: Contract<Http> = self
            .instance_contract(&feed.get_address(pair).unwrap(), proxy_abi)
            .await
            .expect("error creating the proxy instance");
        self.query_contract(&proxy_instance, "accessController", ())
            .await
    }
     */

    pub async fn listen_contract_events(&mut self, contract_address: &str) {
        /*
        let filter = FilterBuilder::default()
        .address(vec![contract.address()])
        .topics(
            Some(vec![hex!(
                "d282f389399565f3671145f5916e51652b60eee8e5c759293a2f5771b8ddfd2e"
            )
            .into()]),
            None,
            None,
            None,
        )
        .build();
         */

        let filter = FilterBuilder::default()
            .address(vec![Address::from_str(contract_address).unwrap()])
            .topics(None, None, None, None)
            .build();

        let sub: SubscriptionStream<WebSocket, Log> = self
            .web3web_socket
            .eth_subscribe()
            .subscribe_logs(filter)
            .await
            .unwrap();
        sub.for_each(|log| async {
            let l: Log = log.unwrap();
            println!("Address: {:?}", l.transaction_hash.unwrap());
            println!("Data: {:?}", l.data);
            println!("Data0: {:?}", l.data.0);
            println!("{}", std::str::from_utf8(&l.data.0).unwrap());
            println!("topics: {:?}", l.topics);
            println!("log_type: {:?}", l.log_type);

            let tx = self
                .web3http
                .eth()
                .transaction(TransactionId::Hash(l.transaction_hash.unwrap()))
                .await
                .unwrap()
                .unwrap();

            let from_addr = tx.from.unwrap_or(H160::zero());
            let to_addr = tx.to.unwrap_or(H160::zero());
            let value = tx.value;
            let input = tx.input;

            println!("from_addr: {:?}", from_addr);
            println!("to_addr: {:?}", to_addr);
            println!("value: {:?}", value);
            println!("input: {:?}", input);
        })
        .await;
    }

    pub async fn build_contract_events(
        &mut self,
        contract_address: &str,
    ) -> SubscriptionStream<WebSocket, Log> {
        let filter = FilterBuilder::default()
            .address(vec![Address::from_str(contract_address).unwrap()])
            .topics(None, None, None, None)
            .build();

        let sub: SubscriptionStream<WebSocket, Log> = self
            .web3web_socket
            .eth_subscribe()
            .subscribe_logs(filter)
            .await
            .unwrap();
        return sub;
    }

    pub async fn init_pair(&self, lp_address: &str) -> Contract<Http> {
        let lp_pair_abi = include_bytes!("../abi/PancakeLPTokenAbi.json");
        let lp_pair_instance_address = lp_address;
        let lp_pair_instance: Contract<Http> = self
            .instance_contract(lp_pair_instance_address, lp_pair_abi)
            .await
            .expect("error creating the contract instance");
        lp_pair_instance
    }

    pub async fn init_router_factory(&mut self, factory_address: &str) -> Contract<Http> {
        let factory_abi = include_bytes!("../abi/PancakeFactoryAbi.json");
        let factory_instance: Contract<Http> = self
            .instance_contract(factory_address, factory_abi)
            .await
            .expect("error creating the contract instance");
        factory_instance
    }

    pub async fn init_router(&mut self, router_address: &str) -> Contract<Http> {
        //let abi: Abi = load_abi_from_json("factoryabi.json");
        let router_abi = include_bytes!("../abi/PancakeRouterAbi.json");

        let router_instance: Contract<Http> = self
            .instance_contract(router_address, router_abi)
            .await
            .expect("error creating the contract instance");
        router_instance
    }

    pub async fn get_factory_address(&mut self, router_instance: &Contract<Http>) -> String {
        let factory_address: Address = self
            .query_contract(&router_instance, "factory", ())
            .await
            .unwrap();

        w3h::to_string(&factory_address).replace("\"", "")
    }

    pub async fn get_weth_address(&mut self, router_instance: &Contract<Http>) -> String {
        let weth_address: Address = self
            .query_contract(&router_instance, "WETH", ())
            .await
            .unwrap();
        w3h::to_string(&weth_address).replace("\"", "")
    }

    pub async fn token_has_liquidity(&self, lp_pair_factory_instance: Contract<Http>) -> bool {
        let lp_pair_reserves: (Uint, Uint, Uint) = self
            .query_contract(&lp_pair_factory_instance, "getReserves", ())
            .await
            .unwrap();
        lp_pair_reserves.0 > U256::from(0) && lp_pair_reserves.1 > U256::from(0)
    }

    pub async fn find_lp_pair(
        &mut self,
        weth_address: &str,
        factory_address: &str,
        token_address: &str,
    ) -> String {
        let factory_instance = self.init_router_factory(factory_address).await;
        let initial_lp_address = "0x0000000000000000000000000000000000000000";

        let lp_pair_address: H160 = self
            .query_contract(
                &factory_instance,
                "getPair",
                (
                    H160::from_str(weth_address).unwrap(),
                    H160::from_str(token_address).unwrap(),
                ),
            )
            .await
            .unwrap();

        let mut lp_token_address = w3h::to_string(&lp_pair_address).replace("\"", "");
        if lp_token_address == initial_lp_address {
            let lp_pair_address: H160 = self
                .query_contract(
                    &factory_instance,
                    "getPair",
                    (
                        H160::from_str(weth_address).unwrap(),
                        H160::from_str(token_address).unwrap(),
                    ),
                )
                .await
                .unwrap();
            lp_token_address = w3h::to_string(&lp_pair_address).replace("\"", "")
        }
        lp_token_address
    }

    pub async fn get_token_reserves(
        &mut self,
        lp_pair_factory_instance: Contract<Http>,
    ) -> (U256, U256, U256) {
        let lp_pair_reserves: (Uint, Uint, Uint) = self
            .query_contract(&lp_pair_factory_instance, "getReserves", ())
            .await
            .unwrap();
        println!("lp_pair_reserves: {:?}", lp_pair_reserves);
        lp_pair_reserves
    }
}
