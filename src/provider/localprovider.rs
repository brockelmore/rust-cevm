use jsonrpc_core as rpc;
use reqwest::blocking::Client;
use ethers_core::types::*;
use serde::{Deserialize, Serialize};
use url::Url;
use thiserror::Error;
use std::fmt;
use serde_json::Value;

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
        error: JsonRpcError
    },
    /// Was success
    Success {
        /// Result field
        result: R
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
}

impl Provider {
    /// Create new provider
    pub fn new(src: String) -> Self {
        Self {
            client: Client::new(),
            url: Url::parse(&src).unwrap()
        }
    }

    /// Get storage for a particular index at an address
    pub fn get_storage_at(&self, address: H160, index: H256, block: Option<BlockNumber>) -> H256 {
        let address = serialize(&address);
        let index = serialize(&index);
        let block = serialize(&block.unwrap_or(BlockNumber::Latest));

        let request = build_request(
            0,
            "eth_getStorageAt",
            vec![address, index, block],
        );
        let res = self.client.post(self.url.clone()).json(&request).send().expect("provider error, get_storage_at");
        let res = res.json::<Response<H256>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets the bytecode for an address
    pub fn get_code(&self, address: H160, block: Option<BlockNumber>) -> Bytes {
        let address = serialize(&address);
        let block = serialize(&block.unwrap_or(BlockNumber::Latest));

        let request = build_request(
            0,
            "eth_getCode",
            vec![address, block],
        );
        let res = self.client.post(self.url.clone()).json(&request).send().expect("provider error, get_code");
        let res = res.json::<Response<Bytes>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets the balance of an address
    pub fn get_balance(&self, address: H160, block: Option<BlockNumber>) -> U256 {
        let address = serialize(&address);
        let block = serialize(&block.unwrap_or(BlockNumber::Latest));

        let request = build_request(
            0,
            "eth_getBalance",
            vec![address, block],
        );
        let res = self.client.post(self.url.clone()).json(&request).send().expect("provider error, get_balance");
        let res = res.json::<Response<U256>>().unwrap();
        res.data.into_result().unwrap()
    }

    /// Gets the tx count for an address
    pub fn get_transaction_count(&self, address: H160, block: Option<BlockNumber>) -> U256 {
        let address = serialize(&address);
        let block = serialize(&block.unwrap_or(BlockNumber::Latest));

        let request = build_request(
            0,
            "eth_getTransactionCount",
            vec![address, block],
        );
        let res = self.client.post(self.url.clone()).json(&request).send().expect("provider error, get_tx_count");
        let res = res.json::<Response<U256>>().unwrap();
        res.data.into_result().unwrap()
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
