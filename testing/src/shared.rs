use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::compiler::solc_types::SolcOutput;
use service::server::*;
use service::shared::*;
#[allow(non_snake_case)]
use service::EVM::*;

pub enum TestRequest {
    Tests,
    Test(String, String),
    Solc(SolcOutput),
}

impl Message for TestRequest {
    type Result = TestResponse;
}

#[derive(MessageResponse, Serialize, Deserialize)]
pub enum TestResponse {
    Tests(HashMap<String, Vec<String>>),
    Test(EthResponse),
    UnknownError,
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
}

impl Message for CompilerRequest {
    type Result = CompilerResponse;
}

#[derive(MessageResponse, Serialize, Deserialize)]
pub enum CompilerResponse {
    Success,
    Failure(String),
    UnknownError
}
