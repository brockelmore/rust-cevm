#![allow(unused_must_use)]
use actix::prelude::*;
use crate::shared::*;


// use bytes::buf::BufExt;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
// use web3::types::*;
use service::shared::EthRequest;

pub struct TestingApi {
    pub evm: Recipient<EthRequest>,
}

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

impl Actor for TestingApi {
    type Context = Context<Self>;

    /// starts a http server
    fn started(&mut self, ctx: &mut Context<Self>) {
        let evm = self.evm.clone();
        let me = ctx.address();
        let a = async move {
            let addr = "127.0.0.1:2347".parse().unwrap();
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
    me: Addr<TestingApi>,
) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        // (&Method::POST, "/") => evm_process(req, evm, me).await,
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

// pub async fn test_process(
//     req: Request<Body>,
//     evm: Recipient<EthRequest>,
//     _me: Addr<TestingApi>,
// ) -> Result<Response<Body>> {
//     let whole_body = hyper::body::aggregate(req).await?;
//     let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
//     let method: String = serde_json::from_value(data["method"].clone()).unwrap();
//     let res;
//     match method.as_ref() {
//
//     }
//     let res = Response::builder()
//         .body(Body::from(serde_json::to_string(&res)?))
//         .unwrap();
//     Ok(res)
// }

// impl Handler
