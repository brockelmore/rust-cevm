use crate::compiler::solc_types::SolcOutput;
use crate::tester::tester_types::*;
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use web3::types::{H160, H256};
use evm::backend::TxReceipt;

#[allow(non_snake_case)]
#[derive(Message)]
#[rtype(result = "Result<TestResponse, ()>")]
pub enum TestRequest {
    Tests,
    Test(String, String, Option<TestOptions>),
    Solc(SolcOutput),
    Sim(H256, bool, Option<Vec<String>>),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TestOptions {
    pub sender: Option<H160>,
    pub testerIsEOA: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TestEVMResponse {
    pub hash: H256,
    pub data: Option<String>,
    pub logs: Option<Vec<SourcedLog>>,
    pub recs: Option<Vec<TxReceipt>>,
    pub trace: Option<Vec<SourceTrace>>,
}

#[derive(MessageResponse, Serialize, Deserialize, Debug)]
pub enum TestResponse {
    Tests(HashMap<String, Vec<String>>),
    Test(Vec<TestEVMResponse>),
    Sim(Vec<TestEVMResponse>),
    UnknownError,
    Success,
    Failure(String),
}

pub struct OutputComponents {
    pub ast: Option<bool>,
    pub ast_json: Option<bool>,
    pub ast_compact_json: Option<bool>,
    pub asm: Option<bool>,
    pub asm_json: Option<bool>,
    pub opcodes: Option<bool>,
    pub bin: Option<bool>,
    pub bin_runtime: Option<bool>,
    pub abi: Option<bool>,
    pub ir: Option<bool>,
    pub ewasm: Option<bool>,
    pub hashes: Option<bool>,
    pub userdoc: Option<bool>,
    pub devdoc: Option<bool>,
    pub metadata: Option<bool>,
}

pub struct CompileOptions {
    pub optimize: Option<bool>,
    pub optimize_runs: Option<usize>,
    pub combined_json: Option<String>,
    pub outputs: Option<OutputComponents>,
}

pub enum CompilerRequest {
    Compile(String, String, Option<CompileOptions>),
    LoadCompiled(String),
}

impl Message for CompilerRequest {
    type Result = CompilerResponse;
}

#[derive(MessageResponse, Serialize, Deserialize)]
pub enum CompilerResponse {
    Success,
    Failure(String),
    UnknownError,
}
