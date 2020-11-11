use service::server::*;

#[allow(non_snake_case)]
use service::{shared::EthRequest, EVM::*};

use actix::clock::delay_for;
use actix::prelude::*;
use compiler::{solc_types::SolcOutput, Compiler};
use std::time::Duration;
use tester::Tester;
use testing_server::TestingApi;

use std::collections::HashMap;
use web3::types::H160;

pub mod compiler;
pub mod shared;
pub mod tester;
pub mod testing_server;

extern crate solc;

#[actix_rt::main]
async fn main() {
    let (evm, _api) = start_blockchain();
    let (_compiler, _tester, _testing_api) = start_compiler_and_tester(evm.clone().recipient());

    loop {
        delay_for(Duration::from_secs(1000)).await;
    }
}

fn start_blockchain() -> (Addr<EVMService>, Addr<Api>) {
    let provider = "https://fee7372b6e224441b747bf1fde15b2bd.eth.rpc.rivet.cloud/";
    let evm = SyncArbiter::start(1, move || EVMService::new(provider));
    let api = Api {
        evm: evm.clone().recipient(),
    }
    .start();
    (evm, api)
}

fn start_compiler_and_tester(
    evm: Recipient<EthRequest>,
) -> (Addr<Compiler>, Addr<Tester>, Addr<TestingApi>) {
    let tester = Tester {
        evm: evm.clone(),
        sender: H160::zero(),
        compiled: SolcOutput::default(),
        contract_addresses: HashMap::new(),
        contract_addresses_rev: HashMap::new(),
        setup_tests: HashMap::new(),
        sigs: HashMap::new(),
        resolved: Vec::new(),
    }
    .start();

    let compiler = Compiler {
        tester: tester.clone().recipient(),
    }
    .start();

    let api = TestingApi {
        evm,
        compiler: compiler.clone().recipient(),
        tester: tester.clone().recipient(),
    }
    .start();

    (compiler, tester, api)
}
