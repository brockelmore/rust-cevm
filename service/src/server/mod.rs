#![allow(unused_must_use)]
use crate::shared::*;
use actix::prelude::*;

use bytes::buf::BufExt;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};
use web3::types::*;

pub struct Api {
    pub evm: Recipient<EthRequest>,
}

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

impl Actor for Api {
    type Context = Context<Self>;

    /// starts a http server
    fn started(&mut self, ctx: &mut Context<Self>) {
        let evm = self.evm.clone();
        let me = ctx.address();
        let a = async move {
            let addr = "127.0.0.1:2346".parse().unwrap();
            let new_service = make_service_fn(move |_| {
                let e = evm.clone();
                let m = me.clone();
                async {
                    Ok::<_, GenericError>(service_fn(move |req| {
                        // Clone again to ensure that client outlives this closure.
                        response_router(req, e.to_owned(), m.to_owned())
                    }))
                }
            });
            let server = Server::bind(&addr).serve(new_service);
            server.await;
        };
        ctx.spawn(a.into_actor(self));
    }

    /// panics on purpose
    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("api stopped");
        // System::current().stop();
        panic!("got interupt");
    }
}

static NOTFOUND: &[u8] = b"Not Found";

/// Routes http requests
/// # Arguments
/// * `req` - Http request
/// * `dydx` - Recipient to handle requests that need dydx data
/// * `me` - Address of this actor
/// * `block` - Recipient to handle requests that need node data
/// * `med` - Recipient to handle requests that need eth/usd medianizer data
pub async fn response_router(
    req: Request<Body>,
    evm: Recipient<EthRequest>,
    me: Addr<Api>,
) -> Result<Response<Body>> {
    // println!("request: {:?}", req);
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => evm_process(req, evm, me).await,
        (&Method::OPTIONS, "/") => options_process(req).await,
        _ => {
            println!("not found, {:?}", req);
            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(NOTFOUND.into())
                .unwrap())
        }
    }
}

pub async fn options_process(req: Request<Body>) -> Result<Response<Body>> {
    Ok(Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        .header("Access-Control-Allow-Headers", "content-type")
        .status(204)
        .body(Body::default())
        .unwrap())
}

pub async fn evm_process(
    req: Request<Body>,
    evm: Recipient<EthRequest>,
    _me: Addr<Api>,
) -> Result<Response<Body>> {
    let whole_body = hyper::body::aggregate(req).await?;
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
    let method: String = serde_json::from_value(data["method"].clone()).unwrap();
    println!("method: {:?}", data["method"]);
    // let id_num: u64 = serde_json::from_value(data["id"].clone());
    let id = serde_json::from_value::<String>(data["id"].clone()).unwrap_or_else(|_| {
        let e: u64 = serde_json::from_value(data["id"].clone()).unwrap();
        e.to_string()
    });
    // let id: u64 = id.parse().unwrap();
    let mut f;
    let res;
    match method.as_ref() {
        "eth_accounts" => {
            let result = evm.send(EthRequest::eth_accounts).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: "".to_string(),
                },
            })
            .unwrap();
        }
        "eth_blockNumber" => {
            let result = evm.send(EthRequest::eth_blockNumber).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: format!("0x{:x}", res.blockNumber().unwrap()),
                },
            })
            .unwrap();
        }
        "eth_getBalance" => {
            let who: H160 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let block;
            match serde_json::from_value::<U256>(data["params"][1].clone()) {
                Ok(bn) => {
                    block = Some(bn);
                }
                _ => {
                    block = None;
                }
            }
            let result = evm.send(EthRequest::eth_getBalance(who, block)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: format!("0x{:x}", res.balance().unwrap()),
                },
            })
            .unwrap();
        }
        "eth_getStorageAt" => {
            let who: H160 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let slot: U256 = serde_json::from_value(data["params"][1].clone()).unwrap();
            let block;
            match serde_json::from_value::<U256>(data["params"][2].clone()) {
                Ok(bn) => {
                    block = Some(bn);
                }
                _ => {
                    block = None;
                }
            }
            let result = evm
                .send(EthRequest::eth_getStorageAt(who, slot, block))
                .await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: res.storage().unwrap(),
                },
            })
            .unwrap();
        }
        "eth_getTransactionCount" => {
            let who: H160 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let block;
            match serde_json::from_value::<U256>(data["params"][1].clone()) {
                Ok(bn) => {
                    block = Some(bn);
                }
                _ => {
                    block = None;
                }
            }
            let result = evm
                .send(EthRequest::eth_getTransactionCount(who, block))
                .await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: res.tx_count().unwrap(),
                },
            })
            .unwrap();
        }
        "eth_getCode" => {
            let who: H160 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let block;
            match serde_json::from_value::<U256>(data["params"][1].clone()) {
                Ok(bn) => {
                    block = Some(bn);
                }
                _ => {
                    block = None;
                }
            }
            let result = evm.send(EthRequest::eth_getCode(who, block)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: res.code().unwrap(),
                },
            })
            .unwrap();
        }
        "eth_chainId" => {
            let result = evm.send(EthRequest::eth_chainId).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: format!("0x{:x}", res.chainId().unwrap()),
                },
            })
            .unwrap();
        }
        "net_version" => {
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: format!("0x{:x}", U256::from(1)),
                },
            })
            .unwrap();
        }
        "eth_sendTransaction" => {
            println!("{:#x?}", data["params"]);
            let tx: TransactionRequest = serde_json::from_value(data["params"][0].clone()).unwrap();
            let opts;
            match serde_json::from_value::<Vec<String>>(data["params"][1].clone()) {
                Ok(options) => {
                    opts = Some(options);
                }
                _ => {
                    opts = None;
                }
            }
            let result = evm.send(EthRequest::eth_sendTransaction(tx, opts)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: "".to_string(),
                },
            })
            .unwrap();
        }
        "eth_sim" => {
            let tx: H256 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let in_place: bool = serde_json::from_value(data["params"][1].clone()).unwrap();
            let opts;
            match serde_json::from_value::<Vec<String>>(data["params"][2].clone()) {
                Ok(options) => {
                    opts = Some(options);
                }
                _ => {
                    opts = None;
                }
            }
            let result = evm.send(EthRequest::eth_sim(tx, in_place, opts)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: "".to_string(),
                },
            })
            .unwrap();
        }
        "eth_sendRawTransaction" => {
            let tx: String = serde_json::from_value(data["params"][0].clone()).unwrap();
            let tx = hex::decode(&tx[2..]).unwrap();
            let result = evm.send(EthRequest::eth_sendRawTransaction(tx)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: res.tx_hash().unwrap(),
                },
            })
            .unwrap();
        }
        "eth_call" => {
            let tx: TransactionRequest = serde_json::from_value(data["params"][0].clone())
                .unwrap_or_else(|_| {
                    let t: CallRequest = serde_json::from_value(data["params"][0].clone()).unwrap();
                    TransactionRequest {
                        from: H160::zero(),
                        to: t.to,
                        gas: t.gas,
                        gas_price: t.gas_price,
                        value: t.value,
                        data: t.data,
                        nonce: None,
                        condition: None,
                    }
                });
            let block;
            match serde_json::from_value::<U256>(data["params"][1].clone()) {
                Ok(bn) => {
                    block = Some(bn);
                }
                _ => {
                    block = None;
                }
            }
            let result = evm.send(EthRequest::eth_call(tx, block)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: "0x".to_owned() + &hex::encode(res.call().unwrap()),
                },
            })
            .unwrap();
        }
        "eth_getBlockByHash" => {
            let hash: H256 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let txs: bool = serde_json::from_value(data["params"][1].clone()).unwrap_or(false);
            let result = evm.send(EthRequest::eth_getBlockByHash(hash, txs)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            if txs {
                f = serde_json::to_string(&RPCResponse {
                    id,
                    jsonrpc: "2.0".to_string(),
                    data: ResponseData::Success {
                        result: res.block_txs().unwrap(),
                    },
                })
                .unwrap();
            } else {
                f = serde_json::to_string(&RPCResponse {
                    id,
                    jsonrpc: "2.0".to_string(),
                    data: ResponseData::Success {
                        result: res.block_txhashes().unwrap(),
                    },
                })
                .unwrap();
            }
        }
        "eth_getBlockByNumber" => {
            let bn: U256 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let txs: bool = serde_json::from_value(data["params"][1].clone()).unwrap_or(false);
            let result = evm.send(EthRequest::eth_getBlockByNumber(bn, txs)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            if txs {
                f = serde_json::to_string(&RPCResponse {
                    id,
                    jsonrpc: "2.0".to_string(),
                    data: ResponseData::Success {
                        result: res.block_txs().unwrap(),
                    },
                })
                .unwrap();
            } else {
                f = serde_json::to_string(&RPCResponse {
                    id,
                    jsonrpc: "2.0".to_string(),
                    data: ResponseData::Success {
                        result: res.block_txhashes().unwrap(),
                    },
                })
                .unwrap();
            }
        }
        "eth_getTransactionByHash" => {
            let hash: H256 =
                serde_json::from_value(data["params"][0].clone()).unwrap_or_else(|_| {
                    let txhash: String = serde_json::from_value(data["params"][0].clone()).unwrap();
                    txhash[2..].parse().unwrap()
                });
            let result = evm.send(EthRequest::eth_getTransactionByHash(hash)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: res.tx_hash().unwrap(),
                },
            })
            .unwrap();
        }
        "eth_getTransactionReceipt" => {
            let hash: H256 =
                serde_json::from_value(data["params"][0].clone()).unwrap_or_else(|_| {
                    let txhash: String = serde_json::from_value(data["params"][0].clone()).unwrap();
                    println!("hash, {:?}", &txhash[2..]);
                    txhash[2..].parse().unwrap()
                });
            let result = evm.send(EthRequest::eth_getTransactionReceipt(hash)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            let rec = res.clone().tx().unwrap().clone();
            let bh = H256::random();
            let rec = web3::types::TransactionReceipt {
                transaction_hash: hash,
                transaction_index: web3::types::U64::from(0),
                block_hash: Some(bh),
                block_number: Some(web3::types::U64::from(rec.block_number.as_u64())),
                cumulative_gas_used: U256::from(rec.cumulative_gas_used),
                gas_used: Some(U256::from(rec.gas_used)),
                contract_address: {
                    if rec.contract_addresses.len() > 0 {
                        Some(*rec.contract_addresses.iter().next().unwrap())
                    } else {
                        None
                    }
                },
                logs: {
                    rec.logs
                        .iter()
                        .enumerate()
                        .map(|(i, l)| web3::types::Log {
                            address: l.address.clone(),
                            topics: l.topics.clone(),
                            data: web3::types::Bytes(l.data.clone()),
                            block_hash: Some(bh),
                            block_number: Some(web3::types::U64::from(rec.block_number.as_u64())),
                            transaction_hash: Some(hash),
                            transaction_index: Some(web3::types::U64::from(0)),
                            log_index: Some(web3::types::U256::from(i)),
                            transaction_log_index: None,
                            log_type: None,
                            removed: Some(false),
                        })
                        .collect()
                },
                status: Some(web3::types::U64::from(rec.status)),
                logs_bloom: web3::types::H2048::zero(),
                root: None,
            };
            println!("rec {:#?}", rec);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success { result: rec },
            })
            .unwrap();
        }
        "eth_gasPrice" => {
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: U256::from(1),
                },
            })
            .unwrap();
        }
        "eth_getLogs" => {
            let from_block;
            match serde_json::from_value::<U256>(data["params"][0]["fromBlock"].clone()) {
                Ok(bn) => {
                    from_block = Some(bn);
                }
                _ => {
                    from_block = None;
                }
            };

            let to_block;
            match serde_json::from_value::<U256>(data["params"][0]["toBlock"].clone()) {
                Ok(bn) => {
                    to_block = Some(bn);
                }
                _ => {
                    to_block = None;
                }
            };
            let addr: Vec<H160> = serde_json::from_value(data["params"][0]["address"].clone())
                .unwrap_or_else(|_| {
                    let e: H160 =
                        serde_json::from_value(data["params"][0]["address"].clone()).unwrap();
                    vec![e]
                });
            let topics: Vec<H256> =
                serde_json::from_value(data["params"][0]["topics"].clone()).unwrap();
            let result = evm
                .send(EthRequest::eth_getLogs(from_block, to_block, addr, topics))
                .await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
            f = serde_json::to_string(&RPCResponse {
                id,
                jsonrpc: "2.0".to_string(),
                data: ResponseData::Success {
                    result: res.logs().unwrap(),
                },
            })
            .unwrap();
        }
        _ => {
            return Ok(Response::new(Body::from("Not found")));
        }
    }

    let res = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST, GET")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST")
        .body(Body::from(f))
        .unwrap();
    Ok(res)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RPCResponse<T> {
    id: String,
    jsonrpc: String,
    #[serde(flatten)]
    pub data: ResponseData<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ResponseData<R> {
    Success { result: R },
}

// impl<R> ResponseData<R> {
//     /// Consume response and return value
//     pub fn into_result(self) -> Result<R, JsonRpcError> {
//         match self {
//             ResponseData::Success { result } => Ok(result),
//             ResponseData::Error { error } => Err(error),
//         }
//     }
// }
