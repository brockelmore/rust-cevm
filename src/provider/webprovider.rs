// use jsonrpc_core as rpc;
// use wasm_bindgen::prelude::*;
// use wasm_bindgen::JsCast;
// use wasm_bindgen_futures::JsFuture;
// use web_sys::{Request, RequestInit, RequestMode, Response};
// use ethers_core::types::*;
//
/// An http client for interacting with a blockchain
pub struct Provider {
    /// the client
    pub client: String
}
//
// impl Provider {
//     pub fn new(url: String) -> Self {
//         Self {
//             client: lib::BlockingRequestClient(url)
//         }
//     }
//
//     pub fn get_storage_at(&self, address: H160, index: H256, block: Option<BlockNumber>) -> H256 {
//         let address = serialize(&address);
//         let index = serialize(&index);
//         let block = serialize(&block.unwrap_or(BlockNumber::Latest));
//
//         let request = build_request(
//             0,
//             "eth_getStorageAt",
//             vec![address, index, block],
//         );
//         serde_json::from_value(self.client.post(request).expect("provider error, get_storage_at"))
//     }
//
//     pub fn get_code(&self, address: H160, block: Option<BlockNumber>) -> Bytes {
//         let address = serialize(&address);
//         let block = serialize(&block.unwrap_or(BlockNumber::Latest));
//
//         let request = build_request(
//             0,
//             "eth_getCode",
//             vec![address, block],
//         );
//         serde_json::from_value(self.client.post(request).expect("provider error, get_code"))
//     }
//
//     pub fn get_balance(&self, address: H160, block: Option<BlockNumber>) -> U256 {
//         let address = serialize(&address);
//         let block = serialize(&block.unwrap_or(BlockNumber::Latest));
//
//         let request = build_request(
//             0,
//             "eth_getBalance",
//             vec![address, block],
//         );
//         serde_json::from_value(self.client.post(request).expect("provider error, get_balance"))
//     }
//
//     pub fn get_transaction_count(&self, address: H160, block: Option<BlockNumber>) -> U256 {
//         let address = serialize(&address);
//         let block = serialize(&block.unwrap_or(BlockNumber::Latest));
//
//         let request = build_request(
//             0,
//             "eth_getTransactionCount",
//             vec![address, block],
//         );
//         serde_json::from_value(self.client.post(request).expect("provider error, get_tx_count"))
//     }
// }
//
// pub fn serialize<T: serde::Serialize>(t: &T) -> rpc::Value {
//     serde_json::to_value(t).expect("Types never fail to serialize.")
// }
//
// pub fn build_request(id: usize, method: &str, params: Vec<rpc::Value>) -> rpc::Call {
//     rpc::Call::MethodCall(rpc::MethodCall {
//         jsonrpc: Some(rpc::Version::V2),
//         method: method.into(),
//         params: rpc::Params::Array(params),
//         id: rpc::Id::Num(id as u64),
//     })
// }
