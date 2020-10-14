use std::io;
use std::process::ExitStatus;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(io::Error);
    }

    errors {
        ExitStatusNotSuccess(
            command: String,
            exit_status: ExitStatus,
            stdout: String,
            stderr: String
        ) {
            description("command exit status is not success (0)"),
            display("command (`{}`) is not success (0) but `{}`", command, exit_status)
        }
        NoSolidityCompilerFound {
            description("neither `solc` nor `solcjs` are in path"),
            display("neither `solc` nor `solcjs` are in path. please install either `solc` or `solcjs` via https://solidity.readthedocs.io/en/latest/installing-solidity.html")
        }
    }
}
