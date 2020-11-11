#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate lazy_static;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

extern crate regex;
use regex::Regex;

pub mod error;
use error::ResultExt;

/// returns whether `solc` is in path.
///
/// `solc` is the C++ implementation of the solidity compiler.
pub fn is_solc_available() -> bool {
    solc_version().is_ok()
}

/// returns whether `solcjs` is in path.
///
/// `solcjs` is the javascript implementation of the solidity compiler.
pub fn is_solcjs_available() -> bool {
    solcjs_version().is_ok()
}

/// returns the output of `solc --version`.
///
/// more specifically returns the last output line which is the version string.
/// `solc` is the C++ implementation of the solidity compiler.
pub fn solc_version() -> error::Result<String> {
    common_version("solc")
}

/// returns the output of `solcjs --version`.
///
/// more specifically returns the last output line which is the version string.
/// `solcjs` is the javascript implementation of the solidity compiler.
pub fn solcjs_version() -> error::Result<String> {
    common_version("solcjs")
}

/// version code that's common for `solc` and `solcjs`
fn common_version(command_name: &str) -> error::Result<String> {
    let command_output = Command::new(command_name)
        .arg("--version")
        .output()
        .chain_err(|| format!("failed to run `{} --version`", command_name))?;
    if !command_output.status.success() {
        return Err(exit_status(command_name, command_output).into());
    }
    let stdout = String::from_utf8(command_output.stdout)
        .chain_err(|| format!("output from `{} --version` is not utf8", command_name))?;
    let version = stdout
        .lines()
        .last()
        .chain_err(|| format!("output from `{} --version` is empty", command_name))?
        .to_owned();
    Ok(version)
}

/// shells out to either `solc` or `solcjs` (whichever is available in that order)
/// to compile all solidity files in `input_dir_path`
/// into abi and bin files in `output_dir_path`.
pub fn compile_dir<A: AsRef<Path>, B: AsRef<Path>>(
    input_dir_path: A,
    output_dir_path: B,
) -> error::Result<()> {
    solc_compile(&input_dir_path, &output_dir_path)?;
    Ok(())
}

/// shells out to `solc` to compile the single file at
/// `input_file_path` into abi and bin files in `output_dir_path`.
///
/// `solc` is the C++ implementation of the solidity compiler.
pub fn solc_compile<A: AsRef<Path>, B: AsRef<Path>>(
    input_dir_path: A,
    output_dir_path: B,
) -> error::Result<Output> {
    // solc --optimize --optimize-runs 50000 --overwrite --combined-json=abi,bin,bin-runtime,srcmap,srcmap-runtime,ast,metadata /=/
    let mut args = vec![
        "--optimize",
        "--optimize-runs",
        "50000",
        "--overwrite",
        "--combined-json",
        "abi,bin,bin-runtime,srcmap,srcmap-runtime,ast,metadata",
        "--bin",
        "--bin-runtime",
        "--evm-version",
        "istanbul",
        "--output-dir",
        output_dir_path.as_ref().to_str().unwrap(),
    ];
    let contracts = solidity_file_paths(input_dir_path).unwrap();
    for contract in contracts.iter() {
        args.push(contract.to_str().unwrap());
    }

    let command_output = Command::new("solc")
        .args(&args)
        .output()
        .chain_err(|| "failed to run process `solc`")?;

    if !command_output.status.success() {
        return Err(exit_status("solc", command_output).into());
    }

    Ok(command_output)
}

/// shells out to `solcjs` to compile the single file at
/// `input_file_path` into abi and bin files in `output_dir_path`.
///
/// `solcjs` is the javascript implementation of the solidity compiler.
pub fn solcjs_compile<A: AsRef<Path>, B: AsRef<Path>>(
    input_file_path: A,
    output_dir_path: B,
) -> error::Result<Output> {
    let command_output = Command::new("solcjs")
        .arg("--bin")
        .arg("--abi")
        .arg("--overwrite")
        .arg("--optimize")
        .arg("--output-dir")
        .arg(output_dir_path.as_ref())
        .arg(input_file_path.as_ref())
        .output()
        .chain_err(|| "failed to run process `solcjs`")?;

    if !command_output.status.success() {
        return Err(exit_status("solcjs", command_output).into());
    }

    Ok(command_output)
}

/// returns all solidity files in `directory`
pub fn solidity_file_paths<T: AsRef<Path>>(directory: T) -> std::io::Result<Vec<PathBuf>> {
    let mut results = Vec::new();

    for maybe_entry in std::fs::read_dir(directory)? {
        let path = maybe_entry?.path();
        if path.is_dir() {
            results.extend(solidity_file_paths(path)?);
        } else if let Some(extension) = path.extension().map(|x| x.to_os_string()) {
            if extension.as_os_str() == "sol" {
                let srcdir = path;
                results.push(fs::canonicalize(&srcdir)?);
            }
        }
    }

    Ok(results)
}

pub fn input_file_path_to_solcjs_output_name_prefix<A: AsRef<Path>>(
    input_file_path: A,
) -> error::Result<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[:./]").unwrap();
    }

    Ok(format!(
        "{}_",
        RE.replace_all(
            input_file_path.as_ref().to_str().ok_or(format!(
                "input file path `{:?}` must be utf8 but isn't",
                input_file_path.as_ref()
            ))?,
            "_"
        )
    ))
}

/// `solcjs` prefixes output files (one per contract) with the input filename while `solc` does not.
/// rename files so output files can be found in identical places regardless
/// of whether `solcjs` or `solc` is used
///
/// effectively undoes the following line in `solcjs`
/// https://github.com/ethereum/solc-js/blob/43e8fe080686fb9627ee9ff93959e3aa61496d22/solcjs#L117
pub fn rename_solcjs_outputs<A: AsRef<Path>, B: AsRef<Path>>(
    input_file_path: A,
    output_dir_path: B,
) -> error::Result<()> {
    let prefix = input_file_path_to_solcjs_output_name_prefix(&input_file_path)?;

    for maybe_entry in std::fs::read_dir(&output_dir_path)? {
        let src_path = maybe_entry?.path();
        if let Some(file_name) = src_path.file_name().map(|x| x.to_os_string()) {
            let file_name = file_name.to_str().ok_or(format!(
                "file name `{:?}` in dir `{:?}` must be utf8 but isn't",
                file_name,
                output_dir_path.as_ref()
            ))?;
            if !file_name.starts_with(&prefix) {
                continue;
            }

            if let Some(extension) = src_path.extension() {
                if extension != "abi" && extension != "bin" {
                    continue;
                }
            }

            // dst = src path with `prefix` stripped from front of file name
            let dst_path = src_path.with_file_name(&file_name[prefix.len()..]);
            std::fs::rename(src_path, dst_path)?;
        }
    }
    Ok(())
}

/// at the time of writing this is broken for `solcjs`
/// due to issue https://github.com/ethereum/solc-js/issues/126
pub fn standard_json(input_json: &str) -> error::Result<String> {
    let is_solc_available = is_solc_available();

    if !is_solc_available && !is_solcjs_available() {
        return Err(error::ErrorKind::NoSolidityCompilerFound.into());
    }

    let command_name = if is_solc_available { "solc" } else { "solcjs" };

    common_standard_json(command_name, input_json)
}

fn common_standard_json(command_name: &str, input_json: &str) -> error::Result<String> {
    let full_command = format!("{} --standard-json", command_name);

    let mut process = Command::new(command_name)
        .arg("--standard-json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .chain_err(|| format!("failed to spawn process `{}`", &full_command))?;

    {
        let stdin = process
            .stdin
            .as_mut()
            .chain_err(|| format!("failed to open stdin for process `{}`", &full_command))?;

        stdin.write_all(input_json.as_bytes()).chain_err(|| {
            format!(
                "failed to write input json to stdin for process `{}`",
                &full_command
            )
        })?;
    }

    let output = process.wait_with_output().chain_err(|| {
        format!(
            "failed to read output json from stdout for process `{}`",
            &full_command
        )
    })?;

    if !output.status.success() {
        return Err(exit_status(full_command, output).into());
    }

    let output_json = String::from_utf8(output.stdout).chain_err(|| {
        format!(
            "stdout from process `{}` must be utf8 but isn't",
            full_command
        )
    })?;

    Ok(output_json)
}

fn exit_status<T: Into<String>>(command: T, output: Output) -> error::ErrorKind {
    let to_str = |d: Vec<u8>| String::from_utf8(d).unwrap_or_else(|_| "<non-utf8 output>".into());
    error::ErrorKind::ExitStatusNotSuccess(
        command.into(),
        output.status,
        to_str(output.stdout),
        to_str(output.stderr),
    )
}
