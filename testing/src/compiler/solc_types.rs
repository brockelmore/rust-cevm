extern crate glob;
extern crate serde;
extern crate serde_json;
extern crate simple_error;

use serde_json::Value as JsonValue;

use ethabi_next::*;


use std::collections::HashMap;
use std::error::Error;






type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(serde::Serialize, Debug)]
struct SolidityFile {
    content: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
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

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default)]
pub struct SolcOutput {
    #[serde(default)]
    pub contracts: HashMap<String, SolcContract>,
    #[serde(default)]
    pub sources: HashMap<String, SolcSource>,
    // #[serde(default)]
    // errors: Vec<SolcError>,
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
