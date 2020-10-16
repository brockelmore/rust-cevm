extern crate glob;
extern crate serde;
extern crate serde_json;
extern crate simple_error;

use actix::prelude::*;



use serde_json::{Value as JsonValue};

use ethabi_next::*;

use solc;
use std::collections::HashMap;


use std::fs;

use std::path::{Path};
use web3::types::{H160};



pub mod solc_types;

use crate::shared::*;
use solc_types::*;


pub enum CompilerError {
    Compilation(String),
    OutputParsing,
}

pub struct Compiler {
    pub tester: Recipient<TestRequest>
}

impl Actor for Compiler {
    type Context = Context<Self>;
}

impl Handler<CompilerRequest> for Compiler {
    type Result = CompilerResponse;

    fn handle(&mut self, msg: CompilerRequest, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            CompilerRequest::Compile(input, out, _opts) => {
                let solc_output = self.compile(input, out);
                match solc_output {
                    Ok(solc_out) => {
                        self.tester.do_send(TestRequest::Solc(solc_out));
                        CompilerResponse::Success
                    },
                    Err(e) => {
                        CompilerResponse::Failure(e)
                    }
                }
            }
        }
    }
}

impl Compiler {
    pub fn compile(&self, input_dir: String, output_dir: String) -> std::result::Result<SolcOutput, String> {
        if !Path::new(&output_dir).exists() {
            fs::create_dir(output_dir.clone()).unwrap();
        }

        match solc::compile_dir(input_dir, output_dir.clone()) {
            Err(e) => {
                match e {
                    solc::error::Error(a, _b) => {
                        return Err(a.to_string())
                    }
                }

            },
            _ => {}
        };

        if let Ok(file) = fs::read_to_string(output_dir + "/combined.json") {
            if let Ok(mut json) = serde_json::from_str::<JsonValue>(&file) {
                let mut solc_output = SolcOutput {
                    contracts: HashMap::new(),
                    sources: HashMap::new(),
                    contract_addresses: HashMap::new(),
                    contract_addresses_rev: HashMap::new(),
                };

                Self::add_cheat_codes(&mut solc_output);

                Self::fix_typing(&mut json, &mut solc_output);

                Ok(solc_output)
            } else {
                 Err("Malformed combined_json".to_string())
            }
        } else {
            Err("Couldn't read combined_json after compilation. Does it exist in the output dir?".to_string())
        }
    }

    fn add_cheat_codes(solc_output: &mut SolcOutput) {
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
        solc_output.contracts.insert("Cheater".to_string(), hax);
        let addr: H160 = "7109709ECfa91a80626fF3989D68f67F5b1DD12D".parse().unwrap();
        solc_output
            .contract_addresses
            .insert(addr, Some("Cheater".to_string()));
        solc_output
            .contract_addresses_rev
            .insert("Cheater".to_string(), Some(addr));
    }

    fn fix_typing(json: &mut JsonValue, solc_output: &mut SolcOutput) {
        match json.clone() {
            JsonValue::Object(contracts) => {
                for (c_name, val) in contracts.iter() {
                    match val {
                        JsonValue::Object(contract) => {
                            for (c_key, val) in contract.iter() {
                                if c_name == "contracts" {
                                    match val {
                                        JsonValue::Object(inner_contract) => {
                                            for (key, val) in inner_contract.iter() {
                                                match key.as_str() {
                                                    "abi" => match val {
                                                        JsonValue::String(as_s) => {
                                                            json[c_name][c_key][key] =
                                                                serde_json::from_str(&as_s).unwrap();
                                                        }
                                                        _ => {}
                                                    },
                                                    "metadata" => match val {
                                                        JsonValue::String(as_s) => {
                                                            json[c_name][c_key][key] =
                                                                serde_json::from_str(&as_s).unwrap();
                                                        }
                                                        _ => {}
                                                    },
                                                    _ => {}
                                                }
                                            }
                                        }
                                        _ => {}
                                    };

                                    let t: SolcContract =
                                        serde_json::from_value(json[c_name][c_key].clone()).unwrap();
                                    solc_output.contracts.insert(c_key.to_string(), t);
                                } else if c_name == "sources" {
                                    let t: SolcSource =
                                        serde_json::from_value(json[c_name][c_key].clone()).unwrap();
                                    solc_output.sources.insert(c_key.to_string(), t);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };
    }
}
