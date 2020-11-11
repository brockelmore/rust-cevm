use ethabi_next::*;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt;
use tiny_keccak::Keccak;
use web3::types::{H160, H256, U256};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BetterBytes(Vec<u8>);

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BetterH160(H160);

impl fmt::Debug for BetterH160 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", hex::encode(self.0))
    }
}

impl fmt::Debug for BetterBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", hex::encode(self.0.clone()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BetterToken {
    Address(BetterH160),
    FixedBytes(H256),
    Bytes(String),
    Int(U256),
    Uint(U256),
    Bool(bool),
    String(String),
    FixedArray(Vec<BetterToken>),
    Array(Vec<BetterToken>),
    Tuple(Vec<BetterToken>),
}

impl Default for BetterToken {
    fn default() -> BetterToken {
        BetterToken::String(String::new())
    }
}

impl From<Token> for BetterToken {
    fn from(tkn: Token) -> BetterToken {
        match tkn {
            Token::Address(a) => BetterToken::Address(BetterH160(a)),
            Token::FixedBytes(b) => BetterToken::FixedBytes(H256::from_slice(b.as_slice())),
            Token::Bytes(b) => BetterToken::Bytes(hex::encode(b)),
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BetterLogParam {
    /// Decoded log name.
    pub name: String,
    /// Decoded log value.
    pub value: BetterToken,
}

impl From<LogParam> for BetterLogParam {
    fn from(param: LogParam) -> BetterLogParam {
        BetterLogParam {
            name: param.name,
            value: BetterToken::from(param.value),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BetterLog {
    pub params: Vec<BetterLogParam>,
}

impl From<Log> for BetterLog {
    fn from(tkn: Log) -> BetterLog {
        BetterLog {
            params: tkn
                .params
                .iter()
                .map(|lp| BetterLogParam::from(lp.clone()))
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ParsedOrNormalLog {
    Parsed(HashMap<String, BetterToken>),
    NotParsed(evm::backend::Log),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourcedLog {
    pub name: String,
    pub event: String,
    pub log: ParsedOrNormalLog,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct SourceTrace {
    pub name: String,
    pub address: H160,
    pub success: bool,
    pub created: bool,
    pub function: String,
    pub inputs: TokensOrString,
    pub cost: usize,
    pub output: TokensOrString,
    pub logs: Vec<SourcedLog>,
    pub inner: Vec<SourceTrace>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TokensOrString {
    Tokens(Vec<BetterToken>),
    String(String),
}

impl Default for TokensOrString {
    fn default() -> TokensOrString {
        TokensOrString::String(String::new())
    }
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

pub fn fill_signature(name: &str, params: &[ParamType], result: &mut [u8]) {
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

pub fn to_contract_name(full: &str) -> &str {
    let src_strs: Vec<&str> = full.rsplit(':').collect();
    let src = src_strs.first().unwrap().clone();
    src
}

pub fn parse_error(output: String) -> Vec<Token> {
    let error_sig = "08c379a0";
    let mut tokens = Vec::new();
    if output.len() > 8 {
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
            tokens = vec![Token::Bytes(hex::decode(output).unwrap())];
        }
    }
    tokens
}

#[derive(Debug)]
pub struct ABIString {
    pub core: String,
    pub other: Vec<ABIString>,
}

pub fn flatten_tokens_to_strings(tokens: Vec<Token>) -> Vec<ABIString> {
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
        as_strings.push(tmp);
    }
    as_strings
}
