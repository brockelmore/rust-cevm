use service::server::*;
use service::shared::*;
#[allow(non_snake_case)]
use service::EVM::*;

use actix::clock::delay_for;
use actix::prelude::*;
use std::time::Duration;

pub mod compiler;
pub mod shared;
pub mod testing_server;
extern crate solc;
// use compiler;
use evm::executor::CallTrace;
use hash;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use web3::types::*;

#[actix_rt::main]
async fn main() {
    let (evm, api) = start_blockchain();

    let mut solc_output = compiler::compile();

    // println!("{:?}", solc_output);

    let from_addr = "de7e7651Ba5d42C0B0aF45DEC81Dbe817087342D".parse().unwrap();

    let entry = "/home/brock/yamV3/contracts/tests/proposal_round_2/proposal.t.sol:Prop2";

    // println!("{:#?}", solc_output.contracts.keys());

    let wanted_out = solc_output.contracts.clone().get(entry).unwrap().clone();

    let tx = TransactionRequest {
        from: from_addr,
        to: None,
        gas: Some(U256::from(50_000_000)),
        gas_price: Some(U256::from(1)),
        data: Some(Bytes(hex::decode(wanted_out.bin.clone()).unwrap())),
        value: None,
        nonce: None,
        condition: None,
    };
    // let result = evm.send(EthRequest::eth_getBalance(who, block)).await
    let result = evm
        .send(EthRequest::eth_sendTransaction(
            tx,
            Some(vec!["receipt".to_string(), "trace".to_string()]),
        ))
        .await;

    let res = result.unwrap_or(EthResponse::eth_unimplemented);

    let recs = res.clone().tx_receipts().unwrap();
    for rec in recs.iter() {
        for addr in rec.contract_addresses.iter() {
            if !solc_output.contract_addresses.contains_key(addr) {
                match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
            }
        }
    }

    let tx = TransactionRequest {
        from: from_addr,
        to: Some(
            solc_output
                .contract_addresses_rev
                .get(entry)
                .unwrap()
                .unwrap(),
        ),
        gas: Some(U256::from(50_000_000)),
        gas_price: Some(U256::from(1)),
        data: Some(Bytes(
            wanted_out
                .abi
                .function("setUp")
                .unwrap()
                .encode_input(&vec![])
                .unwrap(),
        )),
        value: None,
        nonce: None,
        condition: None,
    };
    // let result = evm.send(EthRequest::eth_getBalance(who, block)).await
    let result = evm
        .send(EthRequest::eth_sendTransaction(
            tx,
            Some(vec!["receipt".to_string(), "trace".to_string()]),
        ))
        .await;

    let res = result.unwrap_or(EthResponse::eth_unimplemented);

    let recs = res.clone().tx_receipts().unwrap();
    for rec in recs.iter() {
        for addr in rec.contract_addresses.iter() {
            if !solc_output.contract_addresses.contains_key(addr) {
                match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
            }
        }
    }

    let tx = TransactionRequest {
        from: solc_output
            .contract_addresses_rev
            .get(entry)
            .unwrap()
            .unwrap(),
        to: Some(
            solc_output
                .contract_addresses_rev
                .get(entry)
                .unwrap()
                .unwrap(),
        ),
        gas: Some(U256::from(50_000_000)),
        gas_price: Some(U256::from(1)),
        data: Some(Bytes(
            wanted_out
                .abi
                .function("test_FullProp")
                .unwrap()
                .encode_input(&vec![])
                .unwrap(),
        )),
        value: None,
        nonce: None,
        condition: None,
    };
    // let result = evm.send(EthRequest::eth_getBalance(who, block)).await
    let result = evm
        .send(EthRequest::eth_sendTransaction(
            tx,
            Some(vec!["receipt".to_string(), "trace".to_string()]),
        ))
        .await;

    let res = result.unwrap_or(EthResponse::eth_unimplemented);

    let recs = res.clone().tx_receipts().unwrap();
    for rec in recs.iter() {
        for addr in rec.contract_addresses.iter() {
            if !solc_output.contract_addresses.contains_key(addr) {
                match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
            }
        }
    }

    let call_addrs = flatten_call_addrs(&solc_output, res.clone().tx_trace().unwrap().clone());
    // println!("call addrs: {:?}", call_addrs);
    for addr in call_addrs.iter() {
        match_created(&mut solc_output, evm.clone().recipient(), *addr).await;
    }

    // println!("og trace: {:#?}", res.clone().tx_trace().unwrap().clone());
    println!(
        "trace: {:#?}",
        solc_output.parse_call_trace(res.tx_trace().unwrap().clone())
    );

    loop {
        delay_for(Duration::from_secs(1000)).await;
    }
}

fn flatten_call_addrs(
    solc_output: &compiler::SolcOutput,
    calltraces: Vec<Box<CallTrace>>,
) -> BTreeSet<H160> {
    let mut addrs = BTreeSet::new();
    for calltrace in calltraces.iter() {
        if !solc_output.contract_addresses.contains_key(&calltrace.addr) {
            addrs.insert(calltrace.addr.clone());
        }
        addrs.append(&mut flatten_call_addrs(
            &solc_output,
            calltrace.inner.clone(),
        ));
    }
    addrs
}

fn sig(full: &str) -> web3::types::Bytes {
    web3::types::Bytes(hash::keccak(full)[..4].to_vec())
}

async fn match_created(
    solc_output: &mut compiler::SolcOutput,
    evm: Recipient<EthRequest>,
    address: H160,
) {
    println!("checking against: {:?}", address);
    let result = evm.send(EthRequest::eth_getCode(address, None)).await;

    let res = result.unwrap_or(EthResponse::eth_unimplemented);

    match res {
        EthResponse::eth_getCode(code) => {
            let c = hex::encode(code);
            let mut src = None;
            for (name, contract) in solc_output.contracts.iter() {
                if contract.bin == c || contract.bin_runtime == c {
                    src = Some(name.clone());
                    solc_output
                        .contract_addresses_rev
                        .insert(name.clone(), Some(address));
                }
            }
            println!("found: {:?}, for {:?}", src, address);
            solc_output.contract_addresses.insert(address, src);
        }
        _ => {}
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
