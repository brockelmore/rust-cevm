#![allow(unused_must_use)]
use actix::prelude::*;
use crate::shared::*;
use std::collections::{BTreeSet, BTreeMap};

use bytes::buf::BufExt;
use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
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
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => evm_process(req, evm, me).await,
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

pub async fn evm_process(
    req: Request<Body>,
    evm: Recipient<EthRequest>,
    me: Addr<Api>,
) -> Result<Response<Body>> {
    let whole_body = hyper::body::aggregate(req).await?;
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
    let method: String = serde_json::from_value(data["method"].clone()).unwrap();
    let res;
    match method.as_ref() {
        "eth_accounts" => {
            let result = evm.send(EthRequest::eth_accounts).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        },
        "eth_blockNumber" => {
            let result = evm.send(EthRequest::eth_blockNumber).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        },
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
            let result = evm.send(EthRequest::eth_getStorageAt(who, slot, block)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
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
            let result = evm.send(EthRequest::eth_getTransactionCount(who, block)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
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
        }
        "eth_sendTransaction" => {
            println!("{:#x?}", data["params"][0]);
            let tx: TransactionRequest = serde_json::from_value(data["params"][0].clone()).unwrap();
            let result = evm.send(EthRequest::eth_sendTransaction(tx)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        }
        "eth_sendRawTransaction" => {
            let tx: Vec<u8> = serde_json::from_value(data["params"][0].clone()).unwrap();
            let result = evm.send(EthRequest::eth_sendRawTransaction(tx)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        }
        "eth_call" => {
            let tx: TransactionRequest = serde_json::from_value(data["params"][0].clone()).unwrap();
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
        }
        "eth_getBlockByHash" => {
            let hash: H256 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let result = evm.send(EthRequest::eth_getBlockByHash(hash, false)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        }
        "eth_getBlockByNumber" => {
            let bn: U256 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let result = evm.send(EthRequest::eth_getBlockByNumber(bn, false)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        }
        "eth_getTransactionByHash" => {
            let hash: H256 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let result = evm.send(EthRequest::eth_getTransactionByHash(hash)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        }
        "eth_getTransactionReceipt" => {
            let hash: H256 = serde_json::from_value(data["params"][0].clone()).unwrap();
            let result = evm.send(EthRequest::eth_getTransactionReceipt(hash)).await;
            res = result.unwrap_or(EthResponse::eth_unimplemented);
        }
        _ => {
            return Ok(Response::new(Body::from("Not found")));
        }
    }
    let res = Response::builder()
        .body(Body::from(serde_json::to_string(&res)?))
        .unwrap();
    Ok(res)
}
