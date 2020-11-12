#![allow(non_camel_case_types)]
use super::shared::*;
use actix::prelude::*;
use evm::{
    backend::*, executor::StackExecutor, provider::localprovider::Provider, Config, Handler,
};
use parity_crypto::publickey::public_to_address;
use std::collections::BTreeMap;
use crate::shared::Action;
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
            chain_id: U256::from(1337),
            block_hashes: Vec::new(),
            block_number: U256::from(block.number.expect("Provider didn't give a block number. Is it working correctly?").as_u64()),
            block_coinbase: H160::random(),
            block_timestamp: block.timestamp,
            block_difficulty: U256::zero(),
            block_gas_limit: U256::from(12500000000000i128),
        };
        let state: BTreeMap<H160, MemoryAccount> = BTreeMap::new();
        let backend = ForkMemoryBackendOwned::new(vicinity, state, provider.to_string());
        let mut config = Config::istanbul();
        config.create_contract_limit = None;
        Self { config, backend }
    }

    pub fn update_vicinity_for_tx(&mut self, msg: &EthRequest, sim: &Option<Transaction>) {
        if let Some(bn) = msg.blockNumber(&sim) {
            if bn < self.backend.vicinity.block_number {
                self.backend.vicinity.block_number = bn;
            }
        }
        if let Some(origin) = msg.origin(&sim) {
            self.backend.vicinity.origin = origin;
        }
        if let Some(gp) = msg.gas_price(&sim) {
            self.backend.vicinity.gas_price = gp;
        }
    }
}

impl actix::prelude::Handler<EthRequest> for EVMService {
    type Result = EthResponse;

    fn handle(&mut self, msg: EthRequest, _ctx: &mut SyncContext<Self>) -> Self::Result {
        // store backup of current state
        let timestamp = self.backend.vicinity.block_timestamp;
        let curr_block = self.backend.vicinity.block_number;

        // get sim if necessary
        let mut sim_tx = None;
        match msg {
            EthRequest::eth_sim(ref tx_hash, ref _in_place, ref _opts) => {
                sim_tx = Some(self.backend.tx(*tx_hash));
                let full_block = self
                    .backend
                    .provider
                    .get_block_by_number(self.backend.vicinity.block_number);
                self.backend.vicinity.block_timestamp = full_block.timestamp;
            }
            _ => {}
        }

        // update vicinity params
        self.update_vicinity_for_tx(&msg, &sim_tx);

        // initialize an executor w/ the backend
        let mut exec = StackExecutor::new(
            &self.backend,
            self.backend.vicinity.block_gas_limit.clone().as_usize(),
            &self.config,
        );

        // default to committing
        let mut commit = true;

        let act = |tx: &TransactionRequest| {
            if tx.to != None {
                Action::Call(tx.to.expect("tx.to not none, but failed to unwrap"))
            } else {
                Action::Create
            }
        };

        let mut as_unverified = |tx: &TransactionRequest, action: &Action| {
            let Bytes(raw) = tx.data.clone().expect("handle EthRequest: tx input data didn't exist in as_unverified");
            UnverifiedTransaction {
                unsigned: SelfTransaction {
                    nonce: tx.nonce.unwrap_or(exec.nonce(tx.from)),
                    gas_price: tx.gas_price.unwrap_or_default(),
                    gas: tx.gas.unwrap_or(self.backend.vicinity.block_gas_limit),
                    action: action.clone(),
                    value: tx.value.unwrap_or(U256::zero()),
                    data: raw,
                },
                v: 0,
                r: U256::one(),
                s: U256::one(),
                hash: H256::zero(),
            }
        };


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
                let action = act(&tx);
                let uv_tx = as_unverified(&tx, &action);
                let uv_tx = uv_tx.compute_hash();
                let hash = uv_tx.hash;

                let data;
                let trace;
                match action {
                    Action::Call(_) => {
                        let (succ, tx_data, tx_trace) = exec.transact_call(
                            hash,
                            tx.from,
                            tx.to.expect("SendTransaction: Action type call, but tx.to was None"),
                            tx.value.unwrap_or(U256::zero()),
                            uv_tx.unsigned.data,
                            tx.gas.unwrap_or(self.backend.vicinity.block_gas_limit).as_usize(),
                        );
                        match succ {
                            evm::ExitReason::Succeed(_) => {}
                            _ => {
                                commit = false;
                            }
                        }
                        data = tx_data;
                        trace = tx_trace;
                    }
                    Action::Create => {
                        let (succ, tx_data, tx_trace) = exec.transact_create(
                            hash,
                            tx.from,
                            tx.value.unwrap_or(U256::zero()), // value: 0 eth
                            uv_tx.unsigned.data,              // data
                            tx.gas.unwrap_or(self.backend.vicinity.block_gas_limit).as_usize(),       // gas_limit
                        );
                        match succ {
                            evm::ExitReason::Succeed(_) => {}
                            _ => {
                                commit = false;
                            }
                        }
                        data = tx_data.unwrap_or(H160::zero()).as_bytes().to_vec();
                        trace = tx_trace;
                    }
                }

                let mut re = EthResponse::eth_sendTransaction {
                    hash,
                    data: None,
                    logs: None,
                    recs: None,
                    trace: None,
                };

                let (tx_data, tx_logs, tx_rec, tx_trace) = match re {
                    EthResponse::eth_sendTransaction {
                        hash: _,
                        data: ref mut tx_data,
                        ref mut logs,
                        ref mut recs,
                        trace: ref mut tx_trace,
                    } => (tx_data, logs, recs, tx_trace),
                    _ => unreachable!(),
                };

                let mut with_logs = false;
                let mut with_return = false;
                let mut with_receipt = false;
                let mut with_trace = false;
                println!("options: {:?}", options);
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
                            "no_commit" => {
                                commit = false;
                            }
                            _ => {}
                        }
                    }
                }
                if with_logs {
                    *tx_logs = Some(exec.logs.clone());
                }
                if with_return {
                    *tx_data = Some(data);
                }
                if with_receipt {
                    *tx_rec = Some(exec.pending_txs.clone());
                }
                if with_trace {
                    *tx_trace = Some(trace);
                }

                re
            }
            EthRequest::eth_sendRawTransaction(bytes) => {
                let tx: UnverifiedTransaction = rlp::decode(&bytes).expect("rlp::decode failed for UnverifiedTransaction. Are you sure the bytes are correctly formed?");
                let hash = tx.hash;
                let sender = public_to_address(&tx.recover_public().expect("Unable to recover public key from tx. Is the signature/tx valid?"));
                match tx.action {
                    crate::shared::Action::Create => {
                        let (succ, _, _) = exec.transact_create(
                            hash,
                            sender,
                            tx.value,          // value: 0 eth
                            tx.data.clone(),   // data
                            tx.gas.as_usize(), // gas_limit
                        );
                        match succ {
                            evm::ExitReason::Succeed(_) => {}
                            _ => {
                                commit = false;
                            }
                        }
                    }
                    crate::shared::Action::Call(addr) => {
                        let (succ, _tx_data, _tr) = exec.transact_call(
                            hash,
                            sender,
                            addr,
                            tx.value,
                            tx.data.clone(),
                            tx.gas.as_usize(),
                        );
                        match succ {
                            evm::ExitReason::Succeed(_) => {}
                            _ => {
                                commit = false;
                            }
                        }
                    }
                }
                EthResponse::eth_sendRawTransaction(hash)
            }
            EthRequest::eth_call(tx, _bn) => {
                let action = act(&tx);
                let uv_tx = as_unverified(&tx, &action);
                let uv_tx = uv_tx.compute_hash();
                let hash = uv_tx.hash;
                let data;
                match action {
                    Action::Call(_) => {
                        let (_succ, tx_data, _tr) = exec.transact_call(
                            hash,
                            tx.from,
                            tx.to.expect("Call: Action type call, but tx.to was None"),
                            tx.value.unwrap_or(U256::zero()),
                            uv_tx.unsigned.data,
                            tx.gas
                                .unwrap_or(self.backend.vicinity.block_gas_limit)
                                .as_usize(),
                        );
                        data = tx_data;
                    }
                    Action::Create => {
                        let (_succ, tx_data, _tr) = exec.transact_create(
                            hash,
                            tx.from,
                            tx.value.unwrap_or(U256::zero()), // value: 0 eth
                            uv_tx.unsigned.data,             // data
                            tx.gas
                                .unwrap_or(self.backend.vicinity.block_gas_limit)
                                .as_usize(), // gas_limit
                        );
                        data = tx_data.expect("Call: Create tx did not return a created address").as_bytes().to_vec();
                    }
                }
                // no commit on call
                commit = false;
                EthResponse::eth_call(data)
            }
            EthRequest::eth_tmpDeploy(tx, _options) => {
                let action = Action::Create;
                let uv_tx = as_unverified(&tx, &action);
                let uv_tx = uv_tx.compute_hash();
                let hash = uv_tx.hash;
                let data;
                let (_succ, tx_data, _tr) = exec.transact_create(
                    hash,
                    tx.from,
                    tx.value.unwrap_or(U256::zero()), // value: 0 eth
                    uv_tx.unsigned.data,             // data
                    tx.gas.unwrap_or(self.backend.vicinity.block_gas_limit).as_usize(),       // gas_limit
                );
                let addr = tx_data.expect("Temporary Deployment: Did not return a created address");
                data = exec.code(addr);
                // no commit, tmp deployment
                commit = false;
                EthResponse::eth_call(data)
            }
            EthRequest::eth_getBlockByNumber(bn, txs) => {
                let mut tmp_bn = bn;
                if bn > self.backend.vicinity.block_number {
                    tmp_bn = self.backend.vicinity.block_number;
                }
                if txs {
                    let mut b = self.backend.provider.get_block_by_number_txs(tmp_bn);
                    b.number = Some(web3::types::U64::from(bn.as_u64()));
                    EthResponse::eth_getBlock(None, Some(b))
                } else {
                    let mut b = self.backend.provider.get_block_by_number(tmp_bn);
                    b.number = Some(web3::types::U64::from(bn.as_u64()));
                    EthResponse::eth_getBlock(Some(b), None)
                }
            }
            EthRequest::eth_getBlockByHash(bh, txs) => {
                if txs {
                    EthResponse::eth_getBlock(
                        None,
                        Some(self.backend.provider.get_block_by_hash_txs(bh)),
                    )
                } else {
                    EthResponse::eth_getBlock(
                        Some(self.backend.provider.get_block_by_hash(bh)),
                        None,
                    )
                }
            }
            EthRequest::eth_chainId => EthResponse::eth_chainId(self.backend.vicinity.chain_id),
            EthRequest::eth_getTransactionReceipt(hash) => {
                EthResponse::eth_getTransactionReceipt(self.backend.tx_receipt(hash))
            }
            EthRequest::eth_sim(_hash, in_place, options) => {
                let tx = sim_tx.expect("Sim: simulated tx not found");

                if in_place {
                    // would need to get txs in block up to tx.index, sim all those, then execute
                    // this one.
                    unimplemented!();
                }

                let data;
                let trace;
                let Bytes(raw) = tx.input;
                if tx.to != None {
                    let (_tx_rec, tx_data, tx_trace) = exec.transact_call(
                        tx.hash,
                        tx.from,
                        tx.to.expect("Sim: tx.to defined, but can't unwrap"),
                        tx.value,
                        raw,
                        tx.gas.as_usize(),
                    );
                    data = tx_data;
                    trace = tx_trace;
                } else {
                    let (_tx_rec, tx_data, tx_trace) = exec.transact_create(
                        tx.hash,
                        tx.from,
                        tx.value,          // value: 0 eth
                        raw,               // data
                        tx.gas.as_usize(), // gas_limit
                    );
                    data = tx_data.unwrap_or(H160::zero()).as_bytes().to_vec();
                    trace = tx_trace;
                }

                let mut re = EthResponse::eth_sendTransaction {
                    hash: tx.hash,
                    data: None,
                    logs: None,
                    recs: None,
                    trace: None,
                };

                let (tx_data, tx_logs, tx_rec, tx_trace) = match re {
                    EthResponse::eth_sendTransaction {
                        hash: _,
                        data: ref mut tx_data,
                        ref mut logs,
                        ref mut recs,
                        trace: ref mut tx_trace,
                    } => (tx_data, logs, recs, tx_trace),
                    _ => unreachable!(),
                };

                let mut with_logs = false;
                let mut with_return = false;
                let mut with_receipt = false;
                let mut with_trace = false;
                println!("options: {:?}", options);
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
                            "no_commit" => {
                                commit = false;
                            }
                            _ => {}
                        }
                    }
                }
                if with_logs {
                    *tx_logs = Some(exec.logs.clone());
                }
                if with_return {
                    *tx_data = Some(data);
                }
                if with_receipt {
                    *tx_rec = Some(exec.pending_txs.clone());
                }
                if with_trace {
                    *tx_trace = Some(trace);
                }
                re
            }
            EthRequest::eth_getLogs(from_bn, to_bn, addr, topics) => {
                EthResponse::eth_getLogs(self.backend.logs(from_bn, to_bn, addr, topics))
            }
            _ => {
                println!("!implemented");
                EthResponse::eth_unimplemented
            }
        };

        // if we are committing to backend, deconstruct, else, only keep fork info
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
        } else {
            // We dont want to destroy forked info even if we dont commit others
            let (applies, logs, recs, created) = exec.deconstruct_fork_only();
            self.backend.apply(
                self.backend.vicinity.block_number,
                applies,
                logs,
                recs,
                created,
                false,
            );
        }

        // increase local block number for front end testing
        self.backend.local_block_num += U256::from(1);

        // reset vicinity to backups
        self.backend.vicinity.block_number = curr_block;
        self.backend.vicinity.block_timestamp = timestamp;
        self.backend.vicinity.chain_id = U256::from(1337);
        to_send
    }
}
