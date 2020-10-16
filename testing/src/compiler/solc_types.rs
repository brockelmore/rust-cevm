extern crate glob;
extern crate serde;
extern crate serde_json;
extern crate simple_error;

use serde_json::{Value as JsonValue};

use ethabi_next::*;
use evm::{backend::memory::TxReceipt, executor::CallTrace};

use std::collections::HashMap;
use std::error::Error;
use std::fmt;



use web3::types::{H160, U256};

use tiny_keccak::Keccak;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone)]
pub struct BetterBytes(Vec<u8>);

#[derive(Clone)]
pub struct BetterH160(H160);

impl fmt::Debug for BetterH160 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", hex::encode(self.0.clone()))
    }
}

impl fmt::Debug for BetterBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", hex::encode(self.0.clone()))
    }
}

#[derive(Clone, Debug)]
pub enum BetterToken {
    Address(BetterH160),
    FixedBytes(BetterBytes),
    Bytes(BetterBytes),
    Int(U256),
    Uint(U256),
    Bool(bool),
    String(String),
    FixedArray(Vec<BetterToken>),
    Array(Vec<BetterToken>),
    Tuple(Vec<BetterToken>),
}

impl From<Token> for BetterToken {
    fn from(tkn: Token) -> BetterToken {
        match tkn {
            Token::Address(a) => BetterToken::Address(BetterH160(a)),
            Token::FixedBytes(b) => BetterToken::FixedBytes(BetterBytes(b)),
            Token::Bytes(b) => BetterToken::Bytes(BetterBytes(b)),
            Token::Int(u) => BetterToken::Int(u),
            Token::Uint(u) => BetterToken::Uint(u),
            Token::Bool(b) => BetterToken::Bool(b),
            Token::String(s) => BetterToken::String(s),
            Token::FixedArray(ts) => {
                let mut tss = Vec::new();
                for t in ts.iter() {
                    tss.push(BetterToken::from(t.clone()));
                }
                BetterToken::FixedArray(tss)
            }
            Token::Array(ts) => {
                let mut tss = Vec::new();
                for t in ts.iter() {
                    tss.push(BetterToken::from(t.clone()));
                }
                BetterToken::FixedArray(tss)
            }
            Token::Tuple(ts) => {
                let mut tss = Vec::new();
                for t in ts.iter() {
                    tss.push(BetterToken::from(t.clone()));
                }
                BetterToken::FixedArray(tss)
            }
        }
    }
}

#[derive(serde::Serialize, Debug)]
struct SolidityFile {
    content: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SolcContract {
    pub bin: String,
    #[serde(rename = "bin-runtime")]
    pub bin_runtime: String,
    pub metadata: JsonValue,
    pub srcmap: String,
    #[serde(rename = "srcmap-runtime")]
    pub srcmap_runtime: String,
    #[serde(skip_serializing)]
    pub abi: Contract,
    pub ast: Option<JsonValue>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct SolcOutput {
    #[serde(default)]
    pub contracts: HashMap<String, SolcContract>,
    #[serde(default)]
    pub sources: HashMap<String, SolcSource>,
    #[serde(skip_serializing)]
    pub contract_addresses: HashMap<H160, Option<String>>,
    #[serde(skip_serializing)]
    pub contract_addresses_rev: HashMap<String, Option<H160>>,
    // #[serde(default)]
    // errors: Vec<SolcError>,
}

#[derive(Debug)]
pub struct SourcedLog {
    pub name: String,
    pub log: Option<Log>,
    pub unknown: Option<evm::backend::Log>,
}

#[derive(Debug)]
pub struct SourceTrace {
    pub name: String,
    pub address: H160,
    pub success: bool,
    pub created: bool,
    pub function: String,
    pub inputs: TokensOrString,
    pub cost: usize,
    pub output: TokensOrString,
    pub inner: Vec<Box<SourceTrace>>,
}

#[derive(Debug)]
pub enum TokensOrString {
    Tokens(Vec<BetterToken>),
    String(String),
}

pub struct Writer;

impl Writer {
    /// Returns string which is a formatted represenation of param.
    pub fn write(param: &ParamType) -> String {
        match *param {
            ParamType::Address => "address".to_owned(),
            ParamType::Bytes => "bytes".to_owned(),
            ParamType::FixedBytes(len) => format!("bytes{}", len),
            ParamType::Int(len) => format!("int{}", len),
            ParamType::Uint(len) => format!("uint{}", len),
            ParamType::Bool => "bool".to_owned(),
            ParamType::String => "string".to_owned(),
            ParamType::FixedArray(ref param, len) => format!("{}[{}]", Writer::write(param), len),
            ParamType::Array(ref param) => format!("{}[]", Writer::write(param)),
            ParamType::Tuple(ref params) => format!(
                "({})",
                params
                    .iter()
                    .map(|ref t| format!("{}", t))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
        }
    }
}

pub fn short_signature(name: &str, params: &[ParamType]) -> [u8; 4] {
    let mut result = [0u8; 4];
    fill_signature(name, params, &mut result);
    result
}

fn fill_signature(name: &str, params: &[ParamType], result: &mut [u8]) {
    let types = params
        .iter()
        .map(Writer::write)
        .collect::<Vec<String>>()
        .join(",");

    let data: Vec<u8> = From::from(format!("{}({})", name, types).as_str());

    let mut sponge = Keccak::new_keccak256();
    sponge.update(&data);
    sponge.finalize(result);
}

impl SolcOutput {
    pub fn parse_events_from_rec(&self, rec: TxReceipt) -> Vec<SourcedLog> {
        let mut logs = Vec::with_capacity(rec.logs.len());
        for log in rec.logs.iter() {
            if let Some(maybe_src) = self.contract_addresses.get(&log.address) {
                if let Some(src) = maybe_src {
                    let contract = self.contracts.get(src).unwrap();
                    let raw_log = RawLog::from((log.topics.clone(), log.data.clone()));
                    let events = contract.abi.events();
                    for event in events {
                        let sig = event.signature();
                        if sig == raw_log.clone().topics[0] {
                            logs.push(SourcedLog {
                                name: src.to_string(),
                                log: Some(event.parse_log(raw_log.clone()).unwrap()),
                                unknown: None,
                            });
                        }
                    }
                } else {
                    logs.push(SourcedLog {
                        name: hex::encode(log.address.as_bytes()),
                        log: None,
                        unknown: Some(log.clone()),
                    });
                }
            }
        }
        logs
    }

    pub fn parse_call_trace(&self, trace: Vec<Box<CallTrace>>) -> Vec<Box<SourceTrace>> {
        let mut traces = Vec::with_capacity(trace.len());
        for t in trace.iter() {
            let mut out_tokens;
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
                                .unwrap();
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

                            traces.push(Box::new(SourceTrace {
                                name: src.to_string(),
                                address: t.addr.clone(),
                                success: t.success,
                                created: t.created,
                                function: f.name.clone(),
                                inputs: TokensOrString::Tokens(tss),
                                cost: t.cost,
                                output: TokensOrString::Tokens(tso),
                                inner: self.parse_call_trace(t.inner.clone()),
                            }));
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

                        traces.push(Box::new(SourceTrace {
                            name: src.to_string(),
                            address: t.addr.clone(),
                            success: t.success,
                            created: t.created,
                            function: t.function.clone(),
                            inputs: TokensOrString::String(t.input.clone()),
                            cost: t.cost,
                            output: out,
                            inner: self.parse_call_trace(t.inner.clone()),
                        }));
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
                    traces.push(Box::new(SourceTrace {
                        name: String::new(),
                        address: t.addr.clone(),
                        success: t.success,
                        created: t.created,
                        function: t.function.clone(),
                        inputs: TokensOrString::String(t.input.clone()),
                        cost: t.cost,
                        output: out,
                        inner: self.parse_call_trace(t.inner.clone()),
                    }));
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
                traces.push(Box::new(SourceTrace {
                    name: String::new(),
                    address: t.addr.clone(),
                    success: t.success,
                    created: t.created,
                    function: t.function.clone(),
                    inputs: TokensOrString::String(t.input.clone()),
                    cost: t.cost,
                    output: out,
                    inner: self.parse_call_trace(t.inner.clone()),
                }));
            }
        }
        traces
    }
}

pub fn to_contract_name(full: &str) -> &str {
    let src_strs: Vec<&str> = full.rsplit(':').collect();
    let src = src_strs.first().unwrap().clone();
    src
}

fn parse_error(output: String) -> Vec<Token> {
    let error_sig = "08c379a0";
    let mut tokens = Vec::new();
    if &output[0..8] == error_sig {
        let err_f = Function {
            name: "Error".to_string(),
            inputs: vec![],
            outputs: vec![Param {
                name: "msg".to_string(),
                kind: ParamType::String,
            }],
            state_mutability: StateMutability::View,
        };
        tokens = err_f
            .decode_output(&hex::decode(&output[8..]).unwrap())
            .unwrap();
    } else {
        tokens = vec![Token::Bytes(hex::decode(output.clone()).unwrap())];
    }
    tokens
}

#[derive(Debug)]
pub struct ABIString {
    pub core: String,
    pub other: Vec<Box<ABIString>>,
}

fn flatten_tokens_to_strings(tokens: Vec<Token>) -> Vec<Box<ABIString>> {
    let mut as_strings = Vec::new();
    for token in tokens.iter() {
        let mut tmp = ABIString {
            core: String::new(),
            other: Vec::new(),
        };
        match token {
            Token::Address(addr) => {
                tmp.core = hex::encode(addr.as_bytes());
            }
            Token::FixedBytes(b) => {
                tmp.core = hex::encode(b);
            }
            Token::Bytes(b) => {
                tmp.core = hex::encode(b);
            }
            Token::Int(u) => {
                let mut bytes = [0; 32];
                u.to_big_endian(&mut bytes);
                tmp.core = hex::encode(bytes);
            }
            Token::Uint(u) => {
                let mut bytes = [0; 32];
                u.to_big_endian(&mut bytes);
                tmp.core = hex::encode(bytes);
            }
            Token::Bool(b) => {
                tmp.core = b.to_string();
            }
            Token::String(s) => {
                tmp.core = s.to_string();
            }
            Token::FixedArray(ts) => {
                tmp.other = flatten_tokens_to_strings(ts.to_vec());
            }
            Token::Array(ts) => {
                tmp.other = flatten_tokens_to_strings(ts.to_vec());
            }
            Token::Tuple(ts) => {
                tmp.other = flatten_tokens_to_strings(ts.to_vec());
            }
        }
        as_strings.push(Box::new(tmp));
    }
    as_strings
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SolcContractEvm {
    bytecode: SolcBytecodeOutput,
    deployed_bytecode: SolcBytecodeOutput,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SolcBytecodeOutput {
    object: String,
    source_map: String,
    // link_references,
    // opcodes,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct SolcSource {
    #[serde(rename = "AST")]
    pub ast: JsonValue,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SolcError {
    severity: String,
    formatted_message: String,
}
