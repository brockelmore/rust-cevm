# CompEVM: Rust Ethereum Virtual Machine Implementation designed for Smart Contract Composability testing

Run all of your tests in parallel, using real mainnet chainstate. Roughly 10x faster than dapp tools (HEVM), and significantly less jank* than ganache. No native debugging, but it comes with stack tracing out of the box. Also some other fun stuff in the ui is planned. Gas usage flamegraphs and the like. Tests should be written in solidity for now, but isn't necessary long term. But seriously, write your tests in solidity.


* Kind of depends on where you want your jank. Right now, interfacing with the underlying is jank, but the backend is fast and reliable. UX is next priority.

### Build

To start working with CEVM you'll
need to install [rustup](https://www.rustup.rs/), then you can do:

```bash
$ git clone https://github.com/brockelmore/rust-cevm.git
$ cd rust-cevm
$ cargo build --release --all
```

### Usage

Tbh you probably don't want to use this yet.

But...

If you do:

There are a few different packages included. If you are using this for smart contract testing, you'll use the testing package.

CLI is a WIP. Currently, you have to use the frontend. Its dumb, I know, but c'est la vie. My rpc node is hard coded. plz change b4 using

```bash
$ cd ./rust-cevm/testing
$ cargo run --release
```

In your web browser open up `localhost:2347`. Type in the absolute path to the contracts directory, hit compile, then after its done compiling, the test contracts (denoted by `<your contract> .t.sol`) will auto populate the first dropdown. The second dropdown should auto populate with that contract's tests. Hit test. The test will run and load in the stack trace for you to examine.

Expect jank for most of this stuff. Backend is solid, but testing framework isn't and needs work. reach out if you wanna help make testing contracts not suck 



## License

Apache 2.0
