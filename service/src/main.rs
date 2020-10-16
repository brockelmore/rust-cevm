#[allow(non_snake_case)]
pub mod EVM;
pub mod server;
pub mod shared;

use server::*;
use EVM::*;

use actix::clock::delay_for;
use actix::prelude::*;
use std::time::Duration;

extern crate serde_json;

#[actix_rt::main]
async fn main() {
    let provider = "https://fee7372b6e224441b747bf1fde15b2bd.eth.rpc.rivet.cloud/";
    // let provider = "http://localhost:8855";
    let evm_addr = SyncArbiter::start(1, move || EVMService::new(provider));
    let _api = Api {
        evm: evm_addr.recipient(),
    }
    .start();

    loop {
        // println!("in loop");
        delay_for(Duration::from_secs(10)).await;
    }
}
