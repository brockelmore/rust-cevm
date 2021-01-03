use crate::compiler::solc_types::*;
use actix::prelude::*;
#[allow(non_snake_case)]
use evm::{backend::memory::TxReceipt, executor::CallTrace};
use service::shared::*;
use web3::types::{Bytes, TransactionRequest, H160, U256};

use std::collections::{BTreeMap, HashMap};

use crate::shared::*;
// use ethabi_next::*;
use serde_json::Value as JsonValue;

pub mod tester_types;

use ethabi_next::{Contract, Function, Param, ParamType, RawLog, StateMutability};
use tester_types::*;

#[derive(Clone)]
pub struct Tester {
    pub evm: Recipient<EthRequest>,
    pub sender: H160,
    pub compiled: SolcOutput,
    pub contract_addresses: HashMap<H160, Option<String>>,
    pub contract_addresses_rev: HashMap<String, Option<H160>>,
    pub setup_tests: HashMap<String, bool>,
    pub sigs: HashMap<String, String>,
    pub resolved: Vec<EthResponse>,
}

impl Actor for Tester {
    type Context = Context<Self>;
}

impl Tester {
    pub fn get_tests(&self) -> HashMap<String, Vec<String>> {
        let mut tests: HashMap<String, Vec<String>> = HashMap::new();
        for (src, contract) in self.compiled.contracts.iter() {
            if is_tester(src) {
                for (name, _func) in contract.abi.functions.iter() {
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

    pub fn is_deployed(&self, src: &str) -> bool {
        match self.contract_addresses_rev.get(src) {
            Some(maybe_addr) => matches!(maybe_addr, Some(_addr)),
            _ => false,
        }
    }

    pub fn is_setup(&self, src: &str) -> bool {
        match self.setup_tests.get(src) {
            Some(b) => *b,
            _ => false,
        }
    }

    pub async fn deploy(
        sender: H160,
        bytecode: Vec<u8>,
        evm: Recipient<EthRequest>,
    ) -> EthResponse {
        // craft tx
        let tx = TransactionRequest {
            from: sender,
            to: None,
            gas: Some(U256::from(50_000_000)),
            gas_price: Some(U256::from(1)),
            data: Some(Bytes(bytecode)),
            value: None,
            nonce: None,
            condition: None,
        };

        // Tell evm to execute the tx, returning receipt and trace
        let result = evm
            .send(EthRequest::eth_sendTransaction(
                tx,
                Some(vec!["receipt".to_string(), "trace".to_string()]),
            ))
            .await;

        let eth_resp = result.unwrap_or_else(|e| {
            println!("Failed to unwrap deploy result, result: {:?}", e);
            EthResponse::eth_unimplemented
        });

        eth_resp
    }

    pub async fn temp_deploy(
        sender: H160,
        bytecode: Vec<u8>,
        evm: Recipient<EthRequest>,
    ) -> EthResponse {
        let tx = TransactionRequest {
            from: sender,
            to: None,
            gas: Some(U256::from(50_000_000)),
            gas_price: Some(U256::from(1)),
            data: Some(Bytes(bytecode)),
            value: None,
            nonce: None,
            condition: None,
        };

        let result = evm
            .send(EthRequest::eth_tmpDeploy(
                tx,
                Some(vec!["data".to_string()]),
            ))
            .await;

        let eth_resp = result.unwrap_or_else(|e| {
            println!("Failed to unwrap temp deploy result, result: {:?}", e);
            EthResponse::eth_unimplemented
        });

        eth_resp
    }

    pub async fn setup(sender: H160, contract: H160, evm: Recipient<EthRequest>) -> EthResponse {
        let tx = TransactionRequest {
            from: sender,
            to: Some(contract),
            gas: Some(U256::from(50_000_000)),
            gas_price: Some(U256::from(1)),
            data: Some(Bytes(short_signature("setUp", &[]).to_vec())),
            value: None,
            nonce: None,
            condition: None,
        };

        let eth_resp = evm
            .send(EthRequest::eth_sendTransaction(
                tx,
                Some(vec!["receipt".to_string(), "trace".to_string()]),
            ))
            .await;

        let eth_resp = eth_resp.unwrap_or_else(|e| {
            println!("Failed to unwrap setup result, result: {:?}", e);
            EthResponse::eth_unimplemented
        });

        eth_resp
    }

    pub async fn test(
        sender: H160,
        input: Vec<u8>,
        contract: H160,
        evm: Recipient<EthRequest>,
    ) -> EthResponse {
        let tx = TransactionRequest {
            from: sender,
            to: Some(contract),
            gas: Some(U256::from(50_000_000)),
            gas_price: Some(U256::from(1)),
            data: Some(Bytes(input)),
            value: None,
            nonce: None,
            condition: None,
        };
        let eth_resp = evm
            .send(EthRequest::eth_sendTransaction(
                tx,
                Some(vec![
                    "receipt".to_string(),
                    "trace".to_string(),
                    "no_commit".to_string(),
                ]),
            ))
            .await;

        let eth_resp = eth_resp.unwrap_or_else(|e| {
            println!("Failed to unwrap test result, result: {:?}", e);
            EthResponse::eth_unimplemented
        });

        eth_resp
    }

    pub async fn get_code(address: H160, evm: Recipient<EthRequest>) -> EthResponse {
        let result = evm.send(EthRequest::eth_getCode(address, None)).await;
        let res = result.unwrap_or_else(|e| {
            println!("Failed to unwrap get_code result, result: {:?}", e);
            EthResponse::eth_unimplemented
        });
        res
    }

    fn add_cheat_codes(&mut self) {
        let mut hax = SolcContract {
            bin: String::new(),
            bin_runtime: String::new(),
            metadata: JsonValue::default(),
            srcmap: String::new(),
            srcmap_runtime: String::new(),
            abi: Contract {
                constructor: None,
                functions: HashMap::new(),
                events: HashMap::new(),
                receive: false,
                fallback: false,
            },
            ast: None,
        };
        hax.abi.functions.insert(
            "roll".to_string(),
            vec![Function {
                name: "roll".to_string(),
                inputs: vec![Param {
                    name: "time".to_string(),
                    kind: ParamType::Uint(256),
                }],
                outputs: vec![],
                state_mutability: StateMutability::Nonpayable,
            }],
        );
        hax.abi.functions.insert(
            "warp".to_string(),
            vec![Function {
                name: "warp".to_string(),
                inputs: vec![Param {
                    name: "bn".to_string(),
                    kind: ParamType::Uint(256),
                }],
                outputs: vec![],
                state_mutability: StateMutability::Nonpayable,
            }],
        );
        hax.abi.functions.insert(
            "store".to_string(),
            vec![Function {
                name: "store".to_string(),
                inputs: vec![
                    Param {
                        name: "who".to_string(),
                        kind: ParamType::Address,
                    },
                    Param {
                        name: "slot".to_string(),
                        kind: ParamType::FixedBytes(32),
                    },
                    Param {
                        name: "val".to_string(),
                        kind: ParamType::FixedBytes(32),
                    },
                ],
                outputs: vec![],
                state_mutability: StateMutability::Nonpayable,
            }],
        );
        hax.abi.functions.insert(
            "load".to_string(),
            vec![Function {
                name: "load".to_string(),
                inputs: vec![
                    Param {
                        name: "who".to_string(),
                        kind: ParamType::Address,
                    },
                    Param {
                        name: "slot".to_string(),
                        kind: ParamType::FixedBytes(32),
                    },
                ],
                outputs: vec![],
                state_mutability: StateMutability::View,
            }],
        );
        self.compiled.contracts.insert("Cheater".to_string(), hax);
        let addr: H160 = "7109709ECfa91a80626fF3989D68f67F5b1DD12D".parse().unwrap();
        self.contract_addresses
            .insert(addr, Some("Cheater".to_string()));
        self.contract_addresses_rev
            .insert("Cheater".to_string(), Some(addr));
    }
}

#[derive(Clone, Debug)]
pub struct TestInfo {
    pub src: String,
    pub test: String,
    pub testerIsEOA: bool,
    pub sender: H160,
    pub contract: H160,
    pub evm: Recipient<EthRequest>,
    pub bytecode: Option<Vec<u8>>,
    pub is_deployed: bool,
    pub is_setup: bool,
    pub contract_addresses: HashMap<H160, Option<String>>,
    pub contract_addresses_rev: HashMap<String, Option<H160>>,
    pub contracts: HashMap<String, SolcContract>,
    pub setup_tests: HashMap<String, bool>,
    pub sigs: HashMap<String, String>,
    pub results: Vec<TestEVMResponse>,
}

impl TestInfo {
    pub fn parse_events_from_rec(&self, rec: TxReceipt) -> Vec<SourcedLog> {
        let mut logs = Vec::with_capacity(rec.logs.len());
        logs = self.parse_events(rec.logs);
        logs
    }

    pub fn parse_events(&self, logs: Vec<evm::backend::Log>) -> Vec<SourcedLog> {
        let mut ls = Vec::new();
        for log in logs.iter() {
            let encoded = hex::encode(log.topics[0]);
            let event_name = self.sigs.get(&encoded).unwrap_or(&encoded);
            if let Some(maybe_src) = self.contract_addresses.get(&log.address) {
                if let Some(full_src) = maybe_src {
                    let src = to_contract_name(full_src);
                    let contract = self.contracts.get(full_src).unwrap();
                    let raw_log = RawLog::from((log.topics.clone(), log.data.clone()));
                    let events = contract.abi.events();
                    for event in events {
                        let sig = event.signature();
                        if sig == raw_log.clone().topics[0] {
                            let parsed = event.parse_log(raw_log.clone()).unwrap();
                            let mut tss = HashMap::new();
                            for logparam in parsed.params.iter() {
                                tss.insert(
                                    logparam.name.clone(),
                                    BetterToken::from(logparam.value.clone()),
                                );
                            }

                            ls.push(SourcedLog {
                                name: src.to_string(),
                                event: event_name.clone(),
                                log: ParsedOrNormalLog::Parsed(tss),
                            });
                            // break;
                        }
                    }
                } else {
                    ls.push(SourcedLog {
                        name: hex::encode(log.address.as_bytes()),
                        event: event_name.clone(),
                        log: ParsedOrNormalLog::NotParsed(HexLog::from(log.clone())),
                    });
                }
            } else {
                ls.push(SourcedLog {
                    name: hex::encode(log.address.as_bytes()),
                    event: event_name.clone(),
                    log: ParsedOrNormalLog::NotParsed(HexLog::from(log.clone())),
                });
            }
        }
        ls
    }

    pub fn parse_call_trace(&self, trace: Vec<CallTrace>) -> Vec<SourceTrace> {
        let mut traces = Vec::with_capacity(trace.len());
        for t in trace.iter() {
            let mut out_tokens;
            let func_name = self.sigs.get(&t.function).unwrap_or(&t.function);
            if let Some(maybe_src) = self.contract_addresses.get(&t.addr) {
                if let Some(full_src) = maybe_src {
                    let src = to_contract_name(full_src);
                    let contract = self.contracts.get(full_src).unwrap();
                    let funcs = contract.abi.functions();
                    let mut found = false;
                    for f in funcs {
                        let params: Vec<ParamType> =
                            f.inputs.iter().map(|p| p.kind.clone()).collect();
                        let sig = hex::encode(short_signature(&f.name, &params));
                        if sig == t.function.clone() {
                            let tokens = f
                                .decode_input(&hex::decode(t.input.clone()).unwrap())
                                .unwrap_or_else(|_| {
                                    println!("bad, {:?}, {:?}", f, t.input.clone());
                                    panic!("here");
                                });
                            let mut tss = Vec::new();
                            for t in tokens.iter() {
                                tss.push(BetterToken::from(t.clone()));
                            }
                            if !t.success {
                                out_tokens = parse_error(t.output.clone());
                            } else {
                                out_tokens = f
                                    .decode_output(&hex::decode(t.output.clone()).unwrap())
                                    .unwrap();
                            }

                            let mut tso = Vec::new();
                            for t in out_tokens.iter() {
                                tso.push(BetterToken::from(t.clone()));
                            }

                            traces.push(SourceTrace {
                                name: src.to_string(),
                                address: t.addr,
                                success: t.success,
                                created: t.created,
                                function: f.name.clone(),
                                inputs: TokensOrString::Tokens(tss),
                                cost: t.cost,
                                output: TokensOrString::Tokens(tso),
                                logs: self.parse_events(t.logs.clone()),
                                inner: self.parse_call_trace(t.inner.clone()),
                            });
                            found = true;
                        }
                    }
                    if !found {
                        let out;
                        if !t.success {
                            out_tokens = parse_error(t.output.clone());
                            let mut tso = Vec::new();
                            for t in out_tokens.iter() {
                                tso.push(BetterToken::from(t.clone()));
                            }
                            out = TokensOrString::Tokens(tso);
                        } else {
                            out = TokensOrString::String(t.output.clone());
                        }

                        traces.push(SourceTrace {
                            name: src.to_string(),
                            address: t.addr,
                            success: t.success,
                            created: t.created,
                            function: func_name.clone(),
                            inputs: TokensOrString::String(t.input.clone()),
                            cost: t.cost,
                            output: out,
                            logs: self.parse_events(t.logs.clone()),
                            inner: self.parse_call_trace(t.inner.clone()),
                        });
                    }
                } else {
                    let out;
                    if !t.success {
                        out_tokens = parse_error(t.output.clone());
                        let mut tso = Vec::new();
                        for t in out_tokens.iter() {
                            tso.push(BetterToken::from(t.clone()));
                        }
                        out = TokensOrString::Tokens(tso);
                    } else {
                        out = TokensOrString::String(t.output.clone());
                    }
                    traces.push(SourceTrace {
                        name: String::new(),
                        address: t.addr,
                        success: t.success,
                        created: t.created,
                        function: func_name.clone(),
                        inputs: TokensOrString::String(t.input.clone()),
                        cost: t.cost,
                        output: out,
                        logs: self.parse_events(t.logs.clone()),
                        inner: self.parse_call_trace(t.inner.clone()),
                    });
                }
            } else {
                let out;
                if !t.success {
                    out_tokens = parse_error(t.output.clone());
                    let mut tso = Vec::new();
                    for t in out_tokens.iter() {
                        tso.push(BetterToken::from(t.clone()));
                    }
                    out = TokensOrString::Tokens(tso);
                } else {
                    out = TokensOrString::String(t.output.clone());
                }
                traces.push(SourceTrace {
                    name: String::new(),
                    address: t.addr,
                    success: t.success,
                    created: t.created,
                    function: func_name.clone(),
                    inputs: TokensOrString::String(t.input.clone()),
                    cost: t.cost,
                    output: out,
                    logs: self.parse_events(t.logs.clone()),
                    inner: self.parse_call_trace(t.inner.clone()),
                });
            }
        }
        traces
    }

    pub fn from_eth_resp(&self, eth_resp: EthResponse) -> TestEVMResponse {
        match eth_resp {
            EthResponse::eth_sendTransaction {
                hash,
                data,
                logs,
                recs,
                trace,
            } => {
                let mut d = None;
                if let Some(da) = data {
                    d = Some(hex::encode(da));
                }
                let mut l = None;
                if let Some(ls) = logs {
                    l = Some(self.parse_events(ls));
                }
                let mut t = None;
                if let Some(tr) = trace {
                    t = Some(self.parse_call_trace(tr));
                    // println!("trace {:#?}", t);
                }
                TestEVMResponse {
                    hash,
                    data: d,
                    logs: l,
                    recs,
                    trace: t,
                }
            }
            _ => TestEVMResponse::default(),
        }
    }
}

pub fn flatten_call_addrs(
    contract_addresses: &HashMap<H160, Option<String>>,
    calltraces: Vec<CallTrace>,
) -> BTreeMap<H160, Option<String>> {
    let mut addrs = BTreeMap::new();
    for calltrace in calltraces.iter() {
        if !contract_addresses.contains_key(&calltrace.addr) {
            if calltrace.created {
                addrs.insert(calltrace.addr, Some(calltrace.input.clone()));
            } else {
                addrs.insert(calltrace.addr, None);
            }
        }
        addrs.append(&mut flatten_call_addrs(
            contract_addresses,
            calltrace.inner.clone(),
        ));
    }
    addrs
}

impl Handler<TestRequest> for Tester {
    type Result = ResponseActFuture<Self, Result<TestResponse, ()>>;

    fn handle(&mut self, msg: TestRequest, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            TestRequest::Tests => Box::pin(
                async {}
                    .into_actor(self)
                    .map(|_res, act, _ctx| Ok(TestResponse::Tests(act.get_tests()))),
            ),
            TestRequest::Sim(hash, in_place, opts) => {
                let mut t_info = TestInfo {
                    src: String::new(),
                    test: String::new(),
                    testerIsEOA: false,
                    sender: self.sender,
                    contract: H160::zero(),
                    evm: self.evm.clone(),
                    bytecode: None,
                    is_deployed: true,
                    is_setup: true,
                    contract_addresses: self.contract_addresses.clone(),
                    contract_addresses_rev: self.contract_addresses_rev.clone(),
                    contracts: self.compiled.contracts.clone(),
                    setup_tests: self.setup_tests.clone(),
                    sigs: self.sigs.clone(),
                    results: Vec::new(),
                };

                let e = async move {
                    let sim_resp = t_info
                        .evm
                        .send(EthRequest::eth_sim(hash, in_place, opts))
                        .await;
                    t_info.results.push(t_info.from_eth_resp(sim_resp.unwrap()));
                    Ok(TestResponse::Sim(t_info.results))
                };
                let me = e.into_actor(self);
                Box::pin(me)
            }
            TestRequest::Test(src, test, opts) => {
                if self.compiled.contracts.len() == 0 {
                    return Box::pin(
                        futures::future::ok(TestResponse::Failure(
                            "No contracts loaded".to_string(),
                        ))
                        .into_actor(self),
                    );
                }

                let mut isEOA = true;
                if let Some(ops) = opts {
                    if let Some(sender) = ops.sender {
                        self.sender = sender;
                    }
                    if let Some(EOA) = ops.testerIsEOA {
                        isEOA = EOA;
                    }
                }
                println!("isEOA {:?}", isEOA);

                let is_deployed = self.is_deployed(&src);
                let mut contract = H160::zero();
                let mut bytecode = None;
                if !is_deployed {
                    bytecode = Some(
                        hex::decode(self.compiled.contracts.get(&src).unwrap().bin.clone())
                            .unwrap(),
                    );
                } else {
                    contract = self.contract_addresses_rev.get(&src).unwrap().unwrap();
                }

                let t_info = TestInfo {
                    src: src.clone(),
                    test: test.clone(),
                    testerIsEOA: isEOA,
                    sender: self.sender,
                    contract,
                    evm: self.evm.clone(),
                    bytecode,
                    is_deployed,
                    is_setup: self.is_setup(&src),
                    contract_addresses: self.contract_addresses.clone(),
                    contract_addresses_rev: self.contract_addresses_rev.clone(),
                    contracts: self.compiled.contracts.clone(),
                    setup_tests: self.setup_tests.clone(),
                    sigs: self.sigs.clone(),
                    results: Vec::new(),
                };

                let deploy = async move {
                    let mut deployed = None;
                    if !is_deployed {
                        println!("running deployment");
                        deployed = Some(
                            Self::deploy(
                                t_info.sender,
                                t_info.bytecode.clone().unwrap(),
                                t_info.evm.clone(),
                            )
                            .await,
                        );
                    }
                    (deployed, t_info)
                }
                .into_actor(self);

                let setup = |(maybe_deploy_resp, mut t_info): (Option<EthResponse>, TestInfo),
                             act: &mut Self,
                             _ctx2: &mut Context<Self>| {
                    async move {
                        if let Some(deploy_resp) = maybe_deploy_resp {
                            let recs = deploy_resp.clone().tx_receipts().unwrap();
                            let rec = recs.iter().take(1).next().unwrap();
                            for addr in rec.contract_addresses.iter() {
                                if !t_info.contract_addresses.contains_key(addr) {
                                    let code = Self::get_code(*addr, t_info.evm.clone()).await;
                                    let code = hex::encode(code.code().unwrap());
                                    let mut search_src = None;
                                    for (name, contract) in t_info.contracts.iter() {
                                        if contract.bin == code || contract.bin_runtime == code {
                                            search_src = Some(name.clone());
                                            t_info
                                                .contract_addresses_rev
                                                .insert(name.clone(), Some(*addr));
                                            break;
                                        }
                                    }
                                    t_info.contract_addresses.insert(*addr, search_src);
                                }
                            }
                            let call_addrs = flatten_call_addrs(
                                &t_info.contract_addresses,
                                deploy_resp.clone().tx_trace().unwrap(),
                            );
                            for (addr, _maybe_in_code) in call_addrs.iter() {
                                if !t_info.contract_addresses.contains_key(addr) {
                                    let code = Self::get_code(*addr, t_info.evm.clone()).await;
                                    let code = hex::encode(code.code().unwrap());
                                    let mut search_src = None;
                                    for (name, contract) in t_info.contracts.iter() {
                                        if contract.bin == code || contract.bin_runtime == code {
                                            search_src = Some(name.clone());
                                            t_info
                                                .contract_addresses_rev
                                                .insert(name.clone(), Some(*addr));
                                            break;
                                        }
                                    }
                                    t_info.contract_addresses.insert(*addr, search_src);
                                }
                            }
                            t_info.contract = t_info
                                .contract_addresses_rev
                                .get(&t_info.src)
                                .unwrap()
                                .unwrap();
                            t_info.results.push(t_info.from_eth_resp(deploy_resp));
                        }
                        let mut setup = None;
                        if !t_info.is_setup {
                            t_info.setup_tests.insert(t_info.src.clone(), true);
                            let mut sender = t_info.sender;
                            if t_info.testerIsEOA {
                                sender = t_info.contract;
                            }
                            setup = Some(
                                Self::setup(sender, t_info.contract, t_info.evm.clone()).await,
                            );
                        }
                        (setup, t_info)
                    }
                    .into_actor(act)
                };

                let f = deploy.then(setup);

                let test = |(maybe_setup_resp, mut t_info): (Option<EthResponse>, TestInfo),
                            act2: &mut Self,
                            _ctx3: &mut Context<Self>| {
                    async move {
                        if let Some(setup_resp) = maybe_setup_resp {
                            let recs = setup_resp.clone().tx_receipts().unwrap();
                            let rec = recs.iter().take(1).next().unwrap();
                            for addr in rec.contract_addresses.iter() {
                                if !t_info.contract_addresses.contains_key(addr) {
                                    let code = Self::get_code(*addr, t_info.evm.clone()).await;
                                    let code = hex::encode(code.code().unwrap());
                                    let mut search_src = None;
                                    for (name, contract) in t_info.contracts.iter() {
                                        if contract.bin == code || contract.bin_runtime == code {
                                            search_src = Some(name.clone());
                                            t_info
                                                .contract_addresses_rev
                                                .insert(name.clone(), Some(*addr));
                                            break;
                                        }
                                    }
                                    t_info.contract_addresses.insert(*addr, search_src);
                                }
                            }
                            let call_addrs = flatten_call_addrs(
                                &t_info.contract_addresses,
                                setup_resp.clone().tx_trace().unwrap(),
                            );
                            for (addr, _maybe_in_code) in call_addrs.iter() {
                                if !t_info.contract_addresses.contains_key(addr) {
                                    let code = Self::get_code(*addr, t_info.evm.clone()).await;
                                    let code = hex::encode(code.code().unwrap());
                                    let mut search_src = None;
                                    for (name, contract) in t_info.contracts.iter() {
                                        if contract.bin == code || contract.bin_runtime == code {
                                            search_src = Some(name.clone());
                                            t_info
                                                .contract_addresses_rev
                                                .insert(name.clone(), Some(*addr));
                                            break;
                                        }
                                    }
                                    t_info.contract_addresses.insert(*addr, search_src);
                                }
                            }
                            t_info.results.push(t_info.from_eth_resp(setup_resp));
                        }

                        let input = t_info
                            .contracts
                            .get(&src)
                            .unwrap()
                            .abi
                            .function(&test)
                            .unwrap()
                            .encode_input(&[])
                            .unwrap();

                        let mut sender = t_info.sender;
                        if t_info.testerIsEOA {
                            sender = t_info.contract;
                        }
                        let test_res =
                            Self::test(sender, input, t_info.contract, t_info.evm.clone()).await;
                        let call_addrs = flatten_call_addrs(
                            &t_info.contract_addresses,
                            test_res.clone().tx_trace().unwrap(),
                        );
                        for (addr, maybe_in_code) in call_addrs.iter() {
                            if !t_info.contract_addresses.contains_key(addr) {
                                let code = Self::get_code(*addr, t_info.evm.clone()).await;
                                let mut code = hex::encode(code.code().unwrap());
                                if code.is_empty() {
                                    if let Some(in_code) = maybe_in_code {
                                        if !in_code.is_empty() {
                                            let dbytes = hex::decode(in_code).unwrap();
                                            let deployed_bytecode = Self::temp_deploy(
                                                t_info.sender,
                                                dbytes.clone(),
                                                t_info.evm.clone(),
                                            )
                                            .await;
                                            code = hex::encode(deployed_bytecode.call().unwrap());
                                        }
                                    }
                                }
                                let mut search_src = None;
                                for (name, contract) in t_info.contracts.iter() {
                                    if contract.bin == code || contract.bin_runtime == code {
                                        search_src = Some(name.clone());
                                        t_info
                                            .contract_addresses_rev
                                            .insert(name.clone(), Some(*addr));
                                        break;
                                    }
                                }
                                t_info.contract_addresses.insert(*addr, search_src);
                            }
                        }
                        (test_res, t_info)
                    }
                    .into_actor(act2)
                    .map(move |res, act, _ctx| {
                        let test_res = res.0;
                        let mut t_info = res.1;
                        t_info.results.push(t_info.from_eth_resp(test_res));
                        act.contract_addresses = t_info.contract_addresses;
                        act.contract_addresses_rev = t_info.contract_addresses_rev;
                        act.setup_tests = t_info.setup_tests;
                        Ok(TestResponse::Test(t_info.results))
                    })
                };

                let g = f.then(test);

                Box::pin(g)
            }
            TestRequest::Solc(solc) => {
                // let s = solc.clone();
                Box::pin(async move { solc }.into_actor(self).map(|res, act, _ctx| {
                    act.compiled = res;
                    // reset other things
                    act.contract_addresses = HashMap::new();
                    act.contract_addresses_rev = HashMap::new();
                    act.setup_tests = HashMap::new();
                    act.sigs = HashMap::new();
                    for (_src, contract) in act.compiled.contracts.iter() {
                        for (_name, funcs) in contract.abi.functions.iter() {
                            for f in funcs.iter() {
                                let params: Vec<ParamType> =
                                    f.inputs.iter().map(|p| p.kind.clone()).collect();
                                let sig = hex::encode(short_signature(&f.name, &params));
                                act.sigs.insert(sig, f.name.clone());
                            }
                        }
                        for (_name, events) in contract.abi.events.iter() {
                            for e in events.iter() {
                                act.sigs.insert(hex::encode(e.signature()), e.name.clone());
                            }
                        }
                    }
                    act.resolved = Vec::new();
                    act.add_cheat_codes();
                    Ok(TestResponse::Success)
                }))
            }
        }
    }
}

pub fn is_tester(src: &str) -> bool {
    let src_strs: Vec<&str> = src.rsplit(':').collect();
    let file_name = src_strs.last().unwrap().clone();
    let src: Vec<&str> = file_name.rsplit('.').collect();
    src.iter().any(|c| *c == "t")
}

pub fn is_test(src: &str) -> bool {
    if src.len() > 3 {
        &src[0..4] == "test"
    } else {
        false
    }

}

pub fn is_fail_test(src: &str) -> bool {
    let src_strs: Vec<&str> = src.rsplit(':').collect();
    let file_name = src_strs.last().unwrap().clone();
    let src: Vec<&str> = file_name.rsplit('.').collect();
    src.iter().any(|c| *c == "t")
}
