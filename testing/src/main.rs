#[allow(non_snake_case)]
use service::EVM::*;
use service::server::*;
use service::shared::*;


use std::time::Duration;
use actix::clock::delay_for;
use actix::prelude::*;

// pub mod compiler;
pub mod shared;
pub mod testing_server;
extern crate solc;
// use compiler;
use std::fs;
use std::path::Path;

#[actix_rt::main]
async fn main() {
    // start_blockchain();

    // compiler::run();
    if !Path::new("./out2").exists() {
        fs::create_dir("./out2").unwrap();
    }

    solc::compile_dir("./contracts", "./out2").unwrap();
    println!("done");



    loop {
        delay_for(Duration::from_secs(1000)).await;
    }
}

fn start_blockchain() {
    let provider = "https://fee7372b6e224441b747bf1fde15b2bd.eth.rpc.rivet.cloud/";
    let evm_addr = SyncArbiter::start(1, move || EVMService::new(provider));
    let _api = Api {
        evm: evm_addr.recipient()
    }.start();
}
