#![allow(non_camel_case_types)]
use super::shared::*;
use actix::prelude::*;
use evm::{
    backend::*,
    executor::{CallTrace, StackExecutor},
    provider::localprovider::Provider,
    Config, Handler,
};
use parity_crypto::publickey::public_to_address;
use std::collections::BTreeMap;
use web3::types::*;

#[derive(Clone)]
pub struct EVMService {
    pub config: Config,
    pub backend: ForkMemoryBackendOwned, // pub exec: StackExecutorOwned<ForkMemoryBackendOwned>
}

impl Actor for EVMService {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut SyncContext<Self>) {}

    fn stopped(&mut self, _: &mut SyncContext<Self>) {}
}

impl EVMService {
    pub fn new(provider: &str) -> Self {
        let p = Provider::new(provider.to_string());
        let block = p.get_block();
        let vicinity = MemoryVicinity {
            gas_price: U256::from(5),
            origin: H160::random(),
            chain_id: U256::from(1001),
            block_hashes: Vec::new(),
            block_number: U256::from(block.number.unwrap().as_u64()),
            block_coinbase: H160::random(),
            block_timestamp: block.timestamp,
            block_difficulty: U256::zero(),
            block_gas_limit: U256::from(12500000000000i128),
        };
        let state: BTreeMap<H160, MemoryAccount> = BTreeMap::new();
        let backend = ForkMemoryBackendOwned::new(vicinity.clone(), state, provider.to_string());
        let mut config = Config::istanbul();
        config.create_contract_limit = None;
        Self { config, backend }
    }
}

impl actix::prelude::Handler<EthRequest> for EVMService {
    type Result = EthResponse;

    fn handle(&mut self, msg: EthRequest, _ctx: &mut SyncContext<Self>) -> Self::Result {
        // update gas price
        let curr_block = self.backend.vicinity.block_number.clone();
        match msg {
            EthRequest::eth_getBalance(_who, ref bn) => match bn {
                Some(block) => {
                    self.backend.vicinity.block_number = block.clone();
                }
                _ => {}
            },
            EthRequest::eth_getStorageAt(_who, _slot, ref bn) => match bn {
                Some(block) => {
                    self.backend.vicinity.block_number = block.clone();
                }
                _ => {}
            },
            EthRequest::eth_getTransactionCount(_who, ref bn) => match bn {
                Some(block) => {
                    self.backend.vicinity.block_number = block.clone();
                }
                _ => {}
            },
            EthRequest::eth_getCode(_who, ref bn) => match bn {
                Some(block) => {
                    self.backend.vicinity.block_number = block.clone();
                }
                _ => {}
            },
            EthRequest::eth_sendTransaction(ref tx, ref _opts) => {
                self.backend.vicinity.origin = tx.from;
                self.backend.vicinity.gas_price = tx.gas_price.unwrap_or(U256::from(1));
            }
            EthRequest::eth_sendRawTransaction(ref bytes) => {
                let tx: UnverifiedTransaction = rlp::decode(&bytes).unwrap();
                let sender = public_to_address(&tx.recover_public().unwrap());
                self.backend.vicinity.gas_price = tx.unsigned.gas_price;
                self.backend.vicinity.origin = sender;
            }
            EthRequest::eth_call(ref tx, ref bn) => {
                self.backend.vicinity.gas_price = tx.gas_price.unwrap_or(U256::from(1));
                self.backend.vicinity.origin = tx.from;
                match bn {
                    Some(block) => {
                        self.backend.vicinity.block_number = block.clone();
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        let mut exec = StackExecutor::new(
            &self.backend,
            self.backend.vicinity.block_gas_limit.clone().as_usize(),
            &self.config,
        );

        let mut commit = true;
        let to_send = match msg {
            EthRequest::eth_accounts => EthResponse::eth_accounts(Vec::new()),
            EthRequest::eth_blockNumber => EthResponse::eth_blockNumber(exec.block_number()),
            EthRequest::eth_getBalance(who, _bn) => EthResponse::eth_getBalance(exec.balance(who)),
            EthRequest::eth_getStorageAt(who, slot, _bn) => {
                let mut bytes = [0; 32];
                slot.to_big_endian(&mut bytes);
                EthResponse::eth_getStorageAt(exec.storage(who, H256::from(bytes)))
            }
            EthRequest::eth_getTransactionCount(who, _bn) => {
                EthResponse::eth_getTransactionCount(exec.nonce(who))
            }
            EthRequest::eth_getCode(who, _bn) => EthResponse::eth_getCode(exec.code(who)),
            EthRequest::eth_sendTransaction(tx, options) => {
                let action;
                if tx.to != None {
                    action = crate::shared::Action::Call(tx.to.unwrap());
                } else {
                    action = crate::shared::Action::Create;
                }
                let Bytes(raw) = tx.data.unwrap();
                let selftx = UnverifiedTransaction {
                    unsigned: SelfTransaction {
                        nonce: tx.nonce.unwrap_or(exec.nonce(tx.from)),
                        gas_price: tx.gas_price.unwrap(),
                        gas: tx.gas.unwrap(),
                        action,
                        value: tx.value.unwrap_or(U256::zero()),
                        data: raw,
                    },
                    v: 0,
                    r: U256::one(),
                    s: U256::one(),
                    hash: H256::zero(),
                };
                let selftx = selftx.compute_hash();
                let hash = selftx.hash;
                let data;
                let trace;
                if tx.to != None {
                    let (_r, d, tr) = exec.transact_call(
                        hash,
                        tx.from,
                        tx.to.unwrap(),
                        tx.value.unwrap_or(U256::zero()),
                        selftx.unsigned.data,
                        tx.gas.unwrap().as_usize(),
                    );
                    data = d;
                    trace = tr;
                } else {
                    let (_r, d, tr) = exec.transact_create(
                        hash,
                        tx.from,
                        tx.value.unwrap_or(U256::zero()), // value: 0 eth
                        selftx.unsigned.data,             // data
                        tx.gas.unwrap().as_usize(),       // gas_limit
                    );
                    data = d.unwrap_or(H160::zero()).as_bytes().to_vec();
                    trace = tr;
                }

                let mut re = EthResponse::eth_sendTransaction {
                    hash,
                    data: None,
                    logs: None,
                    recs: None,
                    trace: None,
                };

                let (d, l, r, t) = match re {
                    EthResponse::eth_sendTransaction {
                        hash,
                        data: ref mut d,
                        ref mut logs,
                        ref mut recs,
                        trace: ref mut tr,
                    } => (d, logs, recs, tr),
                    _ => unreachable!(),
                };

                let mut with_logs = false;
                let mut with_return = false;
                let mut with_receipt = false;
                let mut with_trace = false;

                if let Some(opts) = options {
                    for option in opts.into_iter() {
                        match &*option {
                            "logs" => {
                                with_logs = true;
                            }
                            "return" => {
                                with_return = true;
                            }
                            "receipt" => {
                                with_receipt = true;
                            }
                            "trace" => {
                                with_trace = true;
                            }
                            _ => {}
                        }
                    }
                }
                if with_logs {
                    *l = Some(exec.logs.clone());
                }
                if with_return {
                    *d = Some(data);
                }
                if with_receipt {
                    *r = Some(exec.pending_txs.clone());
                }
                if with_trace {
                    *t = Some(trace);
                }
                re
            }
            EthRequest::eth_sendRawTransaction(bytes) => {
                let tx: UnverifiedTransaction = rlp::decode(&bytes).unwrap();
                let hash = tx.hash;
                let sender = public_to_address(&tx.recover_public().unwrap());
                match tx.action {
                    crate::shared::Action::Create => {
                        exec.transact_create(
                            hash,
                            sender,
                            tx.value,          // value: 0 eth
                            bytes,             // data
                            tx.gas.as_usize(), // gas_limit
                        );
                    }
                    crate::shared::Action::Call(addr) => {
                        exec.transact_call(hash, sender, addr, tx.value, bytes, tx.gas.as_usize());
                    }
                }
                EthResponse::eth_sendRawTransaction(hash)
            }
            EthRequest::eth_call(tx, _bn) => {
                let action;
                if tx.to != None {
                    action = crate::shared::Action::Call(tx.to.unwrap());
                } else {
                    action = crate::shared::Action::Create;
                }
                let Bytes(raw) = tx.data.unwrap();
                let selftx = UnverifiedTransaction {
                    unsigned: SelfTransaction {
                        nonce: tx.nonce.unwrap_or(exec.nonce(tx.from)),
                        gas_price: tx.gas_price.unwrap(),
                        gas: tx.gas.unwrap(),
                        action,
                        value: tx.value.unwrap(),
                        data: raw,
                    },
                    v: 0,
                    r: U256::one(),
                    s: U256::one(),
                    hash: H256::zero(),
                };
                let selftx = selftx.compute_hash();
                let hash = selftx.hash;
                let data;
                if tx.to != None {
                    let (_r, d, tr) = exec.transact_call(
                        hash,
                        tx.from,
                        tx.to.unwrap(),
                        tx.value.unwrap_or(U256::zero()),
                        selftx.unsigned.data,
                        tx.gas.unwrap().as_usize(),
                    );
                    data = d;
                } else {
                    let (_r, d, tr) = exec.transact_create(
                        hash,
                        tx.from,
                        tx.value.unwrap_or(U256::zero()), // value: 0 eth
                        selftx.unsigned.data,             // data
                        tx.gas.unwrap().as_usize(),       // gas_limit
                    );
                    data = d.unwrap().as_bytes().to_vec();
                }
                commit = false;
                EthResponse::eth_call(data)
            }
            // EthRequest::eth_getBlockByHash(hash, txs) => {
            //     return EthResponse::eth_getBlockByHash;
            // }
            // EthRequest::eth_getBlockByNumber(bn, txs) => {
            //
            //     return EthResponse::eth_getBlockByNumber();;
            // }
            // EthRequest::eth_getTransactionByHash(hash) => {
            //
            //     return EthResponse::eth_getTransactionByHash;
            // }
            EthRequest::eth_getTransactionReceipt(hash) => {
                EthResponse::eth_getTransactionReceipt(self.backend.tx_receipt(hash))
            }
            _ => {
                println!("!implemented");
                EthResponse::eth_unimplemented
            }
        };

        if commit {
            let (applies, logs, recs, created) = exec.deconstruct();
            self.backend.apply(
                self.backend.vicinity.block_number,
                applies,
                logs,
                recs,
                created,
                false,
            );
        }

        self.backend.vicinity.block_number = curr_block;

        to_send
    }
}
