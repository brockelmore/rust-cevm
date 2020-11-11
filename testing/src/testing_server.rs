#![allow(unused_must_use)]
use crate::shared::*;
use actix::prelude::*;

use bytes::buf::BufExt;

use flate2::{write::ZlibEncoder, Compression};
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Method, Request, Response, Server, StatusCode, Uri};
use service::shared::{EthRequest, EthResponse};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use web3::types::H256;

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
        (&Method::POST, "/load_compiled") => load_compile_process(req, compiler).await,
        (&Method::POST, "/compile") => compile_process(req, compiler).await,
        (&Method::POST, "/test") => test_process(req, tester).await,
        (&Method::POST, "/sim") => sim_process(req, tester).await,
        (&Method::GET, "/tests") => test_request(req, tester).await,
        (&Method::GET, path) => home_request(req).await,
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

pub async fn home_request(req: Request<Body>) -> Result<Response<Body>> {
    let path = local_path_for_request(
        req.uri(),
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(&PathBuf::from("./src/frontend/public/")),
    )
    .unwrap();
    // println!("path: {:?}", path);
    let mime_type = file_path_mime(&path);
    let file_contents = std::fs::read_to_string(path)?;
    // println!("file contents: {:?}", file_contents);
    string_handler(
        &file_contents,
        &(mime_type.type_().as_str().to_owned() + "/" + mime_type.subtype().as_str()),
        None,
    )
    .await
}

pub async fn bytes_handler(
    body: &[u8],
    content_type: &str,
    status: Option<StatusCode>,
) -> Result<Response<Body>> {
    // Compress
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(body).unwrap();
    let compressed = e.finish().unwrap();
    // println!("content_type: {:?}", content_type);
    // Return response
    Ok(Response::builder()
        .status(status.unwrap_or_default())
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_ENCODING, "deflate")
        .body(Body::from(compressed))
        .unwrap())
}

/// Pass string through to bytes_handler
pub async fn string_handler(
    body: &str,
    content_type: &str,
    status: Option<StatusCode>,
) -> Result<Response<Body>> {
    bytes_handler(body.as_bytes(), content_type, status).await
}

/// Map the request's URI to a local path
fn local_path_for_request(uri: &Uri, root_dir: &Path) -> Option<PathBuf> {
    let request_path = uri.path();

    // Trim off the url parameters starting with '?'
    let end = request_path.find('?').unwrap_or(request_path.len());
    let request_path = &request_path[0..end];

    // Convert %-encoding to actual values
    let decoded = percent_encoding::percent_decode_str(&request_path);
    let request_path = if let Ok(p) = decoded.decode_utf8() {
        p
    } else {
        println!("non utf-8 URL: {}", request_path);
        return None;
    };

    // Append the requested path to the root directory
    let mut path = root_dir.to_owned();
    if request_path.starts_with('/') {
        path.push(&request_path[1..]);
    } else {
        println!("found non-absolute path {}", request_path);
        return None;
    }

    // println!("path {:?}, root_dir: {:?}", path, root_dir);
    if path == root_dir {
        return Some(root_dir.join(PathBuf::from("index.html")));
    }

    // println!("URL · path : {} · {}", uri, path.display());

    Some(path)
}

fn file_path_mime(file_path: &Path) -> mime::Mime {
    mime_guess::from_path(file_path).first_or_octet_stream()
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

    let results = compiler
        .send(CompilerRequest::Compile(input_dir, output_dir, None))
        .await;
    let results = results.unwrap_or(CompilerResponse::UnknownError);
    let res = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST, GET")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST")
        .body(Body::from(serde_json::to_string(&results)?))
        .unwrap();
    Ok(res)
}

pub async fn load_compile_process(
    req: Request<Body>,
    compiler: Recipient<CompilerRequest>,
) -> Result<Response<Body>> {
    let whole_body = hyper::body::aggregate(req).await?;
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
    let output_dir: String = serde_json::from_value(data["output_dir"].clone())?;

    let results = compiler
        .send(CompilerRequest::LoadCompiled(output_dir))
        .await;
    let results = results.unwrap_or(CompilerResponse::UnknownError);
    let res = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST, GET")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST")
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
    println!("{:?}", data);
    let method: HashMap<String, Vec<String>> =
        serde_json::from_value(data["tests"].clone()).unwrap();
    let opts;
    match serde_json::from_value::<TestOptions>(data["options"].clone()) {
        Ok(ops) => {
            opts = Some(ops);
        }
        _ => {
            opts = None;
        }
    }

    let mut results: HashMap<String, HashMap<String, TestResponse>> = HashMap::new();
    for (src, tests) in method.iter() {
        let mut res_for_src: HashMap<String, TestResponse> = HashMap::new();
        for test in tests.iter() {
            let res = tester
                .send(TestRequest::Test(
                    src.to_string(),
                    test.to_string(),
                    opts.clone(),
                ))
                .await;
            let res = res.unwrap_or(Ok(TestResponse::UnknownError));
            res_for_src.insert(test.to_string(), res.unwrap());
        }
        results.insert(src.to_string(), res_for_src);
    }

    let res = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST, GET")
        .body(Body::from(serde_json::to_string(&results)?))
        .unwrap();
    Ok(res)
}

pub async fn sim_process(
    req: Request<Body>,
    tester: Recipient<TestRequest>,
) -> Result<Response<Body>> {
    println!("sim");
    let whole_body = hyper::body::aggregate(req).await?;
    let data: serde_json::Value = serde_json::from_reader(whole_body.reader())?;
    println!("{:?}", data);

    let tx: H256 = serde_json::from_value(data["hash"].clone()).unwrap();
    let in_place: bool = serde_json::from_value(data["in_place"].clone()).unwrap();
    let opts;
    match serde_json::from_value::<Vec<String>>(data["options"].clone()) {
        Ok(options) => {
            opts = Some(options);
        }
        _ => {
            opts = None;
        }
    }
    let res = tester.send(TestRequest::Sim(tx, in_place, opts)).await;
    let res = res.unwrap_or(Ok(TestResponse::UnknownError));

    let res = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST, GET")
        .body(Body::from(serde_json::to_string(&res)?))
        .unwrap();
    Ok(res)
}

pub async fn test_request(
    _req: Request<Body>,
    tester: Recipient<TestRequest>,
) -> Result<Response<Body>> {
    let res = tester.send(TestRequest::Tests).await;
    let res = res.unwrap_or(Ok(TestResponse::UnknownError));
    let res = Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "OPTIONS, POST, GET")
        .body(Body::from(serde_json::to_string(&res)?))
        .unwrap();
    Ok(res)
}

// impl Handler

#[derive(Debug)]
struct MyError(String);

impl MyError {
    fn new(msg: &str) -> MyError {
        MyError(msg.to_string())
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for MyError {
    fn description(&self) -> &str {
        &self.0
    }
}
