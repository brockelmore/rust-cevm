# rust-solc

*this is a work in progress. it works as described in the example. the API might change frequently in the future*

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/paritytech/rust_solc.svg?branch=master
[travis-url]: https://travis-ci.org/paritytech/rust_solc

**easily compile solidity files from rust**

shells out to `solc` or `solcjs` (whichever is available in that order).

either `solc` (C++) or `solcjs` (JS) must be installed and in `$PATH`.  
[click here to see how to install `solc`](https://solidity.readthedocs.io/en/latest/installing-solidity.html#binary-packages)  
[click here to see how to install `solcjs`](https://solidity.readthedocs.io/en/latest/installing-solidity.html#npm-node-js)

```rust
extern crate solc;

fn main() {
    let input_directory = "./contracts";
    let output_directory = "./contracts";

    // first tries solc
    // then tries solcjs
    // returns error if no compiler available
    solc::compile_dir(&input_directory, &output_directory).unwrap();

    // now `./contracts` contains a `*.bin` and a `*.abi` file
    // for every contract found in `*.sol` file in `./contracts`
}
```

this is an early version that likely misses features.
[open an issue if you're missing something](https://github.com/snd/rust_solc/issues/new)
