#![allow(unused_must_use)]
use crate::shared::*;
use actix::prelude::*;

use bytes::buf::BufExt;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};

use service::shared::EthRequest;
use std::collections::HashMap;

pub struct TestingApi {
    pub evm: Recipient<EthRequest>,
    pub compiler: Recipient<CompilerRequest>,
    pub tester: Recipient<TestRequest>,
}

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

impl Actor for TestingApi {
    type Context = Context<Self>;

    /// starts a http server
    fn started(&mut self, ctx: &mut Context<Self>) {
        let tester = self.tester.clone();
        let evm = self.evm.clone();
        let compiler = self.compiler.clone();
        let me = ctx.address();
        let a = async move {
            let addr = "127.0.0.1:2347".parse().unwrap();
            let new_service = make_service_fn(move |_| {
                let c = compiler.clone();
                let e = evm.clone();
                let m = me.clone();
                let t = tester.clone();
                async {
                    Ok::<_, GenericError>(service_fn(move |req| {
                        // Clone again to ensure that client outlives this closure.
                        response_router(req, t.to_owned(), c.to_owned(), e.to_owned(), m.to_owned())
                    }))
                }
            });
            let server = Server::bind(&addr).serve(new_service);
            server.await;
        };
        ctx.spawn(a.into_actor(self));
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("api stopped");
    }
}

static NOTFOUND: &[u8] = b"Not Found";

/// Routes http requests
pub async fn response_router(
    req: Request<Body>,
    tester: Recipient<TestRequest>,
    compiler: Recipient<CompilerRequest>,
    _evm: Recipient<EthRequest>,
    _me: Addr<TestingApi>,
) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/compile") => compile_process(req, compiler).await,
        (&Method::POST, "/test") => test_process(req, tester).await,
        (&Method::GET, "/tests") => test_request(req, tester).await,
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

pub async fn compile_process(
    req: Request<Body>,
    compiler: Recipient<CompilerRequest>,
) -> Result<Response<Body>> {
    let whole_body = hyper::body::aggregate(req).await?;
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
    let input_dir: String = serde_json::from_value(data["input_dir"].clone())?;
    let output_dir: String = serde_json::from_value(data["output_dir"].clone())?;
    // let opts: CompileOptions = serde_json::from_value(data["options"].clone()).unwrap();

    let results = compiler.send(CompilerRequest::Compile(input_dir, output_dir, None)).await;
    let results = results.unwrap_or(CompilerResponse::UnknownError);
    let res = Response::builder()
        .body(Body::from(serde_json::to_string(&results)?))
        .unwrap();
    Ok(res)
}


pub async fn test_process(
    req: Request<Body>,
    tester: Recipient<TestRequest>,
) -> Result<Response<Body>> {
    let whole_body = hyper::body::aggregate(req).await?;
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
    let method: HashMap<String, Vec<String>> = serde_json::from_value(data["tests"].clone()).unwrap();

    let mut results: HashMap<String, HashMap<String, TestResponse>> = HashMap::new();
    for (src, tests) in method.iter() {
        let mut res_for_src: HashMap<String, TestResponse> = HashMap::new();
        for test in tests.iter() {
            let res = tester.send(TestRequest::Test(src.to_string(), test.to_string())).await;
            let res = res.unwrap_or(TestResponse::UnknownError);
            res_for_src.insert(test.to_string(), res);
        }
        results.insert(src.to_string(), res_for_src);
    }

    let res = Response::builder()
        .body(Body::from(serde_json::to_string(&results)?))
        .unwrap();
    Ok(res)
}

pub async fn test_request(
    _req: Request<Body>,
    tester: Recipient<TestRequest>,
) -> Result<Response<Body>> {
    let res = tester.send(TestRequest::Tests).await;
    let res = res.unwrap_or(TestResponse::UnknownError);
    let res = Response::builder()
        .body(Body::from(serde_json::to_string(&res)?))
        .unwrap();
    Ok(res)
}

// impl Handler
