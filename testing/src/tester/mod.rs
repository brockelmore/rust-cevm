use actix::prelude::*;
use service::server::*;
use service::shared::*;
#[allow(non_snake_case)]
use service::EVM::*;

use evm::executor::CallTrace;

use crate::shared::*;
use crate::compiler::solc_types::*;
use web3::types::H160;

use std::collections::{BTreeSet, HashMap};


pub struct Tester {
    pub evm: Recipient<EthRequest>,
    pub compiled: SolcOutput
}


impl Actor for Tester {
    type Context = Context<Self>;
}

impl Tester {
    pub fn get_tests(&self) -> HashMap<String, Vec<String>> {
        let mut tests: HashMap<String, Vec<String>> = HashMap::new();
        for (src, contract) in self.compiled.contracts.iter() {
            if is_tester(src) {
                for (name, func) in contract.abi.functions.iter() {
                    if is_test(&name) {
                        if let Some(curr_tests) = tests.get_mut(src) {
                            curr_tests.push(name.to_string());
                            *curr_tests = curr_tests.clone();
                        } else {
                            tests.insert(src.clone(), vec![name.clone()]);
                        }
                    }
                }
            }
        }
        tests
    }
}


pub fn is_tester(src: &str) -> bool {
    let mut src_strs: Vec<&str> = src.rsplit(':').collect();
    let mut file_name = src_strs.last().unwrap().clone();
    let src: Vec<&str> = file_name.rsplit('.').collect();
    src.iter().any(|c| *c == "t")
}

pub fn is_test(src: &str) -> bool {
    &src[0..4] == "test"
}

pub fn is_fail_test(src: &str) -> bool {
    let mut src_strs: Vec<&str> = src.rsplit(':').collect();
    let mut file_name = src_strs.last().unwrap().clone();
    let src: Vec<&str> = file_name.rsplit('.').collect();
    src.iter().any(|c| *c == "t")
}


pub fn flatten_call_addrs(
    solc_output: &SolcOutput,
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

async fn match_created(
    solc_output: &mut SolcOutput,
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
