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

// let mut solc_output = compiler::compile();

// let tests = compiler::get_tests(solc_output);

// println!("{:#?}", tests);

// let from_addr: H160 = "de7e7651Ba5d42C0B0aF45DEC81Dbe817087342D".parse().unwrap();

// let entry = "/home/brock/yamV3/contracts/tests/proposal_round_2/proposal.t.sol:Prop2";

// println!("{:#?}", solc_output.contracts.keys());

// let wanted_out = solc_output.contracts.clone().get(entry).unwrap().clone();
//
// let tx = TransactionRequest {
//     from: from_addr,
//     to: None,
//     gas: Some(U256::from(50_000_000)),
//     gas_price: Some(U256::from(1)),
//     data: Some(Bytes(hex::decode(wanted_out.bin.clone()).unwrap())),
//     value: None,
//     nonce: None,
//     condition: None,
// };
// // let result = evm.send(EthRequest::eth_getBalance(who, block)).await
// let result = evm
//     .send(EthRequest::eth_sendTransaction(
//         tx,
//         Some(vec!["receipt".to_string(), "trace".to_string()]),
//     ))
//     .await;
//
// let res = result.unwrap_or(EthResponse::eth_unimplemented);
//
// let recs = res.clone().tx_receipts().unwrap();
// for rec in recs.iter() {
//     for addr in rec.contract_addresses.iter() {
//         if !solc_output.contract_addresses.contains_key(addr) {
//             match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
//         }
//     }
// }
//
// let tx = TransactionRequest {
//     from: from_addr,
//     to: Some(
//         solc_output
//             .contract_addresses_rev
//             .get(entry)
//             .unwrap()
//             .unwrap(),
//     ),
//     gas: Some(U256::from(50_000_000)),
//     gas_price: Some(U256::from(1)),
//     data: Some(Bytes(
//         wanted_out
//             .abi
//             .function("setUp")
//             .unwrap()
//             .encode_input(&vec![])
//             .unwrap(),
//     )),
//     value: None,
//     nonce: None,
//     condition: None,
// };
// // let result = evm.send(EthRequest::eth_getBalance(who, block)).await
// let result = evm
//     .send(EthRequest::eth_sendTransaction(
//         tx,
//         Some(vec!["receipt".to_string(), "trace".to_string()]),
//     ))
//     .await;
//
// let res = result.unwrap_or(EthResponse::eth_unimplemented);
//
// let recs = res.clone().tx_receipts().unwrap();
// for rec in recs.iter() {
//     for addr in rec.contract_addresses.iter() {
//         if !solc_output.contract_addresses.contains_key(addr) {
//             match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
//         }
//     }
// }
//
// let tx = TransactionRequest {
//     from: solc_output
//         .contract_addresses_rev
//         .get(entry)
//         .unwrap()
//         .unwrap(),
//     to: Some(
//         solc_output
//             .contract_addresses_rev
//             .get(entry)
//             .unwrap()
//             .unwrap(),
//     ),
//     gas: Some(U256::from(50_000_000)),
//     gas_price: Some(U256::from(1)),
//     data: Some(Bytes(
//         wanted_out
//             .abi
//             .function("test_FullProp")
//             .unwrap()
//             .encode_input(&vec![])
//             .unwrap(),
//     )),
//     value: None,
//     nonce: None,
//     condition: None,
// };
// // let result = evm.send(EthRequest::eth_getBalance(who, block)).await
// let result = evm
//     .send(EthRequest::eth_sendTransaction(
//         tx,
//         Some(vec!["receipt".to_string(), "trace".to_string()]),
//     ))
//     .await;
//
// let res = result.unwrap_or(EthResponse::eth_unimplemented);
//
// let recs = res.clone().tx_receipts().unwrap();
// for rec in recs.iter() {
//     for addr in rec.contract_addresses.iter() {
//         if !solc_output.contract_addresses.contains_key(addr) {
//             match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
//         }
//     }
// }
//
// let call_addrs = flatten_call_addrs(&solc_output, res.clone().tx_trace().unwrap().clone());
// // println!("call addrs: {:?}", call_addrs);
// for addr in call_addrs.iter() {
//     match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
// }
//
// // println!("og trace: {:#?}", res.clone().tx_trace().unwrap().clone());
// println!(
//     "trace: {:#?}",
//     solc_output.parse_call_trace(res.tx_trace().unwrap().clone())
// );
//
// let elapsed = now.elapsed();
// println!("Elapsed: {:?}", elapsed);
