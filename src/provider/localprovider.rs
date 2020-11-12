use crate::backend::memory::TxReceipt;
use ethers_core::types::*;
use jsonrpc_core as rpc;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;
use std::{thread, time};
use thiserror::Error;
use url::Url;

/// Delay between calls
pub static DELAY: u64 = 2;

#[derive(Serialize, Deserialize, Debug, Clone, Error)]
/// A JSON-RPC 2.0 error
pub struct JsonRpcError {
    /// The error code
    pub code: i64,
    /// The error message
    pub message: String,
    /// Additional data
    pub data: Option<Value>,
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(code: {}, message: {}, data: {:?})",
            self.code, self.message, self.data
        )
    }
}

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionReceiptExtended {
    /// caller address.
    #[serde(rename = "from")]
    pub from: H160,
    /// caller address.
    #[serde(rename = "to")]
    pub to: Option<H160>,
    /// Transaction hash.
    #[serde(rename = "transactionHash")]
    pub transaction_hash: H256,
    /// Index within the block.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: usize,
    /// Hash of the block this transaction was included within.
    #[serde(rename = "blockHash")]
    pub block_hash: Option<H256>,
    /// Number of the block this transaction was included within.
    #[serde(rename = "blockNumber")]
    pub block_number: Option<U64>,
    /// Cumulative gas used within the block after this was executed.
    #[serde(rename = "cumulativeGasUsed")]
    pub cumulative_gas_used: U256,
    /// Gas used by this transaction alone.
    ///
    /// Gas used is `None` if the the client is running in light client mode.
    #[serde(rename = "gasUsed")]
    pub gas_used: Option<U256>,
    /// Contract address created, or `None` if not a deployment.
    #[serde(rename = "contractAddress")]
    pub contract_address: Option<H160>,
    /// Logs generated within this transaction.
    pub logs: Vec<web3::types::Log>,
    /// Status: either 1 (success) or 0 (failure).
    pub status: Option<U64>,
    /// State root.
    pub root: Option<H256>,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: web3::types::H2048,
}

/// Response from blockchain
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response<T> {
    id: u64,
    jsonrpc: String,
    /// response data holder
    #[serde(flatten)]
    pub data: ResponseData<T>,
}

/// response enum
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ResponseData<R> {
    /// Was error
    Error {
        /// Error field
        error: JsonRpcError,
    },
    /// Was success
    Success {
        /// Result field
        result: R,
    },
}

impl<R> ResponseData<R> {
    /// Consume response and return value
    pub fn into_result(self) -> Result<R, JsonRpcError> {
        match self {
            ResponseData::Success { result } => Ok(result),
            ResponseData::Error { error } => Err(error),
        }
    }
}

/// A http client that interacts with a blockchain
#[derive(Clone, Debug)]
pub struct Provider {
    /// The client
    pub client: Client,
    /// The provider url
    pub url: Url,
    /// last call
    pub last_call: u128,
}

impl Provider {
    /// Create new provider
    pub fn new(src: String) -> Self {
        Self {
            client: Client::new(),
            url: Url::parse(&src).unwrap(),
            last_call: 0,
        }
    }

    fn check_delay(&self) {
        thread::sleep(time::Duration::from_millis(DELAY));
    }

    /// Get storage for a particular index at an address
    pub fn get_block_number(&self) -> U256 {
        self.check_delay();
        //println!("eth_blockNumber");
        let request = build_request(0, "eth_blockNumber", vec![]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_storage_at");
        let res = res.json::<Response<U256>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Get storage for a particular index at an address
    pub fn get_block(&self) -> Block<H256> {
        self.check_delay();
        //println!("eth_getBlockByNumber");
        let index = serialize(&"latest".to_string());
        let txs = serialize(&false);
        let request = build_request(0, "eth_getBlockByNumber", vec![index, txs]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_storage_at");
        let res = res.json::<Response<Block<H256>>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Get storage for a particular index at an address
    pub fn get_block_by_number_txs(
        &self,
        bn: U256,
    ) -> web3::types::Block<web3::types::Transaction> {
        self.check_delay();
        //println!("eth_getBlockByNumber");
        let index = serialize(&bn);
        let t = serialize(&true);
        let request = build_request(0, "eth_getBlockByNumber", vec![index, t]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_storage_at");

        let res = res
            .json::<Response<web3::types::Block<web3::types::Transaction>>>()
            .unwrap();
        res.data.into_result().unwrap()
    }

    /// Get storage for a particular index at an address
    pub fn get_block_by_number(&self, bn: U256) -> web3::types::Block<web3::types::H256> {
        self.check_delay();
        //println!("eth_getBlockByNumber");
        let index = serialize(&bn);
        let t = serialize(&false);
        let request = build_request(0, "eth_getBlockByNumber", vec![index, t]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_storage_at");

        let res = res
            .json::<Response<web3::types::Block<web3::types::H256>>>()
            .unwrap();
        res.data.into_result().unwrap()
    }

    /// Get storage for a particular index at an address
    pub fn get_block_by_hash_txs(&self, bh: H256) -> web3::types::Block<web3::types::Transaction> {
        self.check_delay();
        //println!("eth_getBlockByHashTxs");
        let index = serialize(&bh);
        let t = serialize(&true);
        let request = build_request(0, "eth_getBlockByHash", vec![index, t]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_storage_at");

        let res = res
            .json::<Response<web3::types::Block<web3::types::Transaction>>>()
            .unwrap();
        res.data.into_result().unwrap()
    }

    /// Get storage for a particular index at an address
    pub fn get_block_by_hash(&self, bh: H256) -> web3::types::Block<web3::types::H256> {
        self.check_delay();
        //println!("eth_getBlockByHash");
        let index = serialize(&bh);
        let t = serialize(&false);
        let request = build_request(0, "eth_getBlockByHash", vec![index, t]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_storage_at");

        let res = res
            .json::<Response<web3::types::Block<web3::types::H256>>>()
            .unwrap();
        res.data.into_result().unwrap()
    }

    /// Get storage for a particular index at an address
    pub fn get_storage_at(&self, address: H160, index: H256, block: Option<U256>) -> H256 {
        self.check_delay();
        //println!("eth_getStorageAt, {:?}, {:?}", address, index);
        let address = serialize(&address);
        let index = serialize(&index);
        let b;
        match block {
            Some(bn) => {
                b = serialize(&bn);
            }
            _ => {
                b = serialize(&BlockNumber::Latest);
            }
        }

        let request = build_request(0, "eth_getStorageAt", vec![address, index, b]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_storage_at");
        let res = res.json::<Response<H256>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets the bytecode for an address
    pub fn get_code(&self, address: H160, block: Option<U256>) -> Bytes {
        if address == "7109709ecfa91a80626ff3989d68f67f5b1dd12d".parse().unwrap() {
            return Bytes::new();
        }
        self.check_delay();
        //println!("eth_getCode, {:?}", address);
        let address = serialize(&address);
        let b;
        match block {
            Some(bn) => {
                b = serialize(&bn);
            }
            _ => {
                b = serialize(&BlockNumber::Latest);
            }
        }

        let request = build_request(0, "eth_getCode", vec![address, b]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_code");
        let res = res.json::<Response<Bytes>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets the balance of an address
    pub fn get_balance(&self, address: H160, block: Option<U256>) -> U256 {
        self.check_delay();
        //println!("eth_getBalance, {:?}", address);
        // //println!("balance block: {:?}", block);
        let address = serialize(&address);
        let b;
        match block {
            Some(bn) => {
                b = serialize(&bn);
            }
            _ => {
                b = serialize(&BlockNumber::Latest);
            }
        }

        let request = build_request(0, "eth_getBalance", vec![address, b]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_balance");
        let res = res.json::<Response<U256>>().unwrap_or({
            let res = self
                .client
                .post(self.url.clone())
                .json(&request)
                .send()
                .expect("provider error, get_balance");
            res.json::<Response<U256>>().expect("I retried, but couldn't unwrap response")
        });
        res.data.into_result().unwrap()
    }

    /// Gets the tx count for an address
    pub fn get_transaction_count(&self, address: H160, block: Option<U256>) -> U256 {
        self.check_delay();
        //println!("eth_getTransactionCount: {:?}", address);
        let address = serialize(&address);
        let b;
        match block {
            Some(bn) => {
                b = serialize(&bn);
            }
            _ => {
                b = serialize(&BlockNumber::Latest);
            }
        }

        let request = build_request(0, "eth_getTransactionCount", vec![address, b]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_tx_count");
        let res = res.json::<Response<U256>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets Tx from hash
    pub fn get_transaction(&self, hash: H256) -> web3::types::Transaction {
        self.check_delay();
        //println!("eth_getTransactionByHash: {:?}", hash);
        let h = serialize(&hash);

        let request = build_request(0, "eth_getTransactionByHash", vec![h]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_tx");
        let res = res.json::<Response<web3::types::Transaction>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets Tx from hash
    pub fn get_logs(
        &self,
        from: U256,
        to: U256,
        addrs: Vec<H160>,
        topics: Vec<H256>,
    ) -> Vec<web3::types::Log> {
        self.check_delay();
        //println!("get_logs: {:?}", addrs);
        let filter = serialize(
            &web3::types::FilterBuilder::default()
                .from_block(web3::types::BlockNumber::Number(web3::types::U64::from(
                    from.as_u64(),
                )))
                .to_block(web3::types::BlockNumber::Number(web3::types::U64::from(
                    to.as_u64(),
                )))
                .address(addrs)
                .topics(Some(topics), None, None, None)
                .build(),
        );
        let request = build_request(0, "eth_getLogs", vec![filter]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_logs");
        let res = res.json::<Response<Vec<web3::types::Log>>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets the tx count for an address
    pub fn get_transaction_receipt(&self, hash: H256) -> TxReceipt {
        self.check_delay();
        //println!("eth_getTransactionReceipt: {:?}", hash);
        let h = serialize(&hash);

        let request = build_request(0, "eth_getTransactionReceipt", vec![h]);
        let res = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .expect("provider error, get_tx_count");
        let res = res.json::<Response<TransactionReceiptExtended>>().unwrap();
        let res = res.data.into_result().unwrap();
        let mut logs = Vec::with_capacity(res.logs.len());
        for log in res.logs.iter() {
            let web3::types::Bytes(raw) = log.data.clone();
            logs.push(crate::backend::Log {
                address: log.address,
                topics: log.topics.clone(),
                data: raw,
            });
        }
        match res.contract_address {
            Some(addr) => {
                let mut addrs = BTreeSet::new();
                addrs.insert(addr);
                TxReceipt {
                    hash: res.transaction_hash,
                    caller: res.from,
                    to: res.to,
                    block_number: U256::from(res.block_number.unwrap().as_u64()),
                    cumulative_gas_used: res.cumulative_gas_used.as_usize(),
                    gas_used: res.gas_used.unwrap().as_usize(),
                    contract_addresses: addrs,
                    logs,
                    status: res.status.unwrap().as_usize(),
                }
            }
            _ => TxReceipt {
                hash: res.transaction_hash,
                caller: res.from,
                to: res.to,
                block_number: U256::from(res.block_number.unwrap().as_u64()),
                cumulative_gas_used: res.cumulative_gas_used.as_usize(),
                gas_used: res.gas_used.unwrap().as_usize(),
                contract_addresses: BTreeSet::new(),
                logs,
                status: res.status.unwrap().as_usize(),
            },
        }
    }
}

/// Serialize http request things
pub fn serialize<T: serde::Serialize>(t: &T) -> rpc::Value {
    serde_json::to_value(t).expect("Types never fail to serialize.")
}

/// Builds RPC call
pub fn build_request(id: usize, method: &str, params: Vec<rpc::Value>) -> rpc::Call {
    rpc::Call::MethodCall(rpc::MethodCall {
        jsonrpc: Some(rpc::Version::V2),
        method: method.into(),
        params: rpc::Params::Array(params),
        id: rpc::Id::Num(id as u64),
    })
}
