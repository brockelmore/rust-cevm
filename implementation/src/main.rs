use ethers_core::types::*;
use evm::{
    backend::*,
    executor::{StackAccount, StackExecutor},
    Config, Handler,
};
use hex;
use primitive_types::*;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args[1]);
    let vicinity = MemoryVicinity {
        gas_price: U256::from(5),
        origin: H160::random(),
        chain_id: U256::from(1001),
        block_hashes: Vec::new(),
        block_number: U256::zero(),
        block_coinbase: H160::random(),
        block_timestamp: U256::zero(),
        block_difficulty: U256::zero(),
        block_gas_limit: U256::from(12500000i128),
    };
    let state: BTreeMap<H160, MemoryAccount> = BTreeMap::new();
    let provider = args[1].clone();
    let mut backend = ForkMemoryBackend::new(&vicinity, state, provider.to_string(), None);
    let mut state: BTreeMap<H160, StackAccount> = BTreeMap::new();

    // let myBytes = Vec::new(); // deployed bytecode

    let deleted: BTreeSet<H160> = BTreeSet::new();
    let config = Config::istanbul();
    let mut exec = StackExecutor::new(&backend, 12500000, &config);
    // example of interacting with non-local data, i.e. getting codesize of weth on-chain
    // println!("code size of weth from chain: {:?}", exec.code_size("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".parse().unwrap()));
    //
    // // it is then stored in local storage so as to not have to call the fork unnecessarily
    // println!("it is now stored locally: {:?}", exec.account_mut("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".parse().unwrap()));

    let my_bytes;
    // read bytecode for contract
    match std::fs::read_to_string("./Sample.bin-runtime") {
        Ok(bytes) => match hex::decode(bytes.clone()) {
            Ok(h) => {
                my_bytes = h;
            }
            Err(e) => {
                my_bytes = hex::decode(
                    bytes
                        .chars()
                        .into_iter()
                        .take(bytes.chars().count() - 1)
                        .collect::<String>(),
                )
                .unwrap();
            }
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!("please run again with appropriate permissions.");
            }
            panic!("{}", e);
        }
    }
    // println!("{:?}", );
    let caller = "d8da6bf26964af9d7eed9e03e53415d37aa96045".parse().unwrap();
    // deploy our contract
    let s = exec.transact_create(
        caller,       // address of vitalik.eth
        U256::zero(), // value: 0 eth
        my_bytes,     // data
        1250000,      // gas_limit
    );
    let my_new_contract;
    match s.1 {
        Some(addr) => {
            my_new_contract = addr;
            println!("new contract: {:?}", exec.account_mut(addr))
        }
        _ => {
            panic!("failed to create");
        }
    }

    // lets call test(), which is just 100*200
    let ret = exec.transact_call(
        caller,
        my_new_contract,
        U256::zero(),
        hex::decode("f8a8fd6d").unwrap(),
        50000,
    );
    // we expect a U256 response, equal to 20000, so decode it below
    println!("ret: {:?}", U256::from(from_slice(&ret.1[..])));

    // lets call getUniPair(), which is just WETH<>USDC pair
    let ret = exec.transact_call(
        caller,
        my_new_contract,
        U256::zero(),
        hex::decode("48c8ec72").unwrap(),
        50000,
    );

    let (applies, logs) = exec.deconstruct();
    backend.apply(applies, logs, false);

    // we expect a H160 response, equal to 0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc, so decode it below
    println!("ret: {:?}", H160::from_slice(&ret.1[12..]));
}

fn from_slice(bytes: &[u8]) -> [u8; 32] {
    let mut array = [0; 32];
    let bytes = &bytes[..array.len()]; // panics if not enough data
    array.copy_from_slice(bytes);
    array
}
