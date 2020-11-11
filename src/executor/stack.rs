use crate::backend::{memory::TxReceipt, Apply, Backend, Basic, Log};
use crate::gasometer::{self, Gasometer};
use crate::{
    Capture, Config, Context, CreateScheme, ExitError, ExitReason, ExitSucceed, ExternalOpcode,
    Handler, Opcode, Runtime, Stack, Transfer,
};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cmp::min;
use core::convert::Infallible;

use primitive_types::{H160, H256, U256};
use sha3::{Digest, Keccak256};

/// Account definition for the stack-based executor.
#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct StackAccount {
    /// Basic account information, including nonce and balance.
    pub basic: Basic,
    /// Code. `None` means the code is currently unknown.
    pub code: Option<Vec<u8>>,
    /// Storage. Not inserted values mean it is currently known, but not empty.
    pub storage: BTreeMap<H256, H256>,
    /// Storage. Not inserted values mean it is currently known, but not empty.
    pub original_storage: BTreeMap<H256, H256>,
    /// Original code: incase code is destroyed
    pub original_code: Option<Vec<u8>>,
    /// Original balances
    pub original_basic: Basic,
    /// Whether the storage in the database should be reset before storage
    /// values are applied.
    pub reset_storage: bool,
    /// Whether the storage in the database should be reset before storage
    /// values are applied.
    pub reset_storage_backend: bool,
}

/// Call trace of a tx
#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallTrace {
    /// Successful
    pub success: bool,
    /// Callee
    pub addr: H160,
    /// Creation
    pub created: bool,
    /// function
    pub function: String,
    /// Input
    pub input: String,
    /// Gas cost
    pub cost: usize,
    /// Output
    pub output: String,
    /// Logs
    pub logs: Vec<Log>,
    /// inner calls
    pub inner: Vec<Box<CallTrace>>,
}

/// Stack-based executor.
#[derive(Clone)]
pub struct StackExecutor<'backend, 'config, B> {
    /// Backend for data
    pub backend: &'backend B,
    /// Config
    pub config: &'config Config,
    /// Gas tracker
    pub gasometer: Gasometer<'config>,
    /// State
    pub state: BTreeMap<H160, StackAccount>,
    /// Deleted Addrs
    pub deleted: BTreeSet<H160>,
    /// emitted logs
    pub logs: Vec<Log>,
    /// Precompile Map
    pub precompiles: BTreeMap<H160, fn(&[u8], Option<usize>)>,
    /// Precompiles
    pub precompile:
        fn(H160, &[u8], Option<usize>) -> Option<Result<(ExitSucceed, Vec<u8>, usize), ExitError>>,
    /// is static flag
    pub is_static: bool,
    /// Recursion depth
    pub depth: Option<usize>,
    /// Txs that are pending
    pub pending_txs: Vec<TxReceipt>,
    /// Txs that are pending
    pub tmp_bn: Option<U256>,
    /// Txs that are pending
    pub tmp_timestamp: Option<U256>,
    /// created contracts
    pub created_contracts: BTreeSet<H160>,
    /// Call trace
    pub call_trace: Vec<Box<CallTrace>>,
}

fn precompiles(
    _address: H160,
    _input: &[u8],
    _target_gas: Option<usize>,
) -> Option<Result<(ExitSucceed, Vec<u8>, usize), ExitError>> {
    None
}

impl<'backend, 'config, B: Backend> StackExecutor<'backend, 'config, B> {
    /// Create a new stack-based executor.
    pub fn new(backend: &'backend B, gas_limit: usize, config: &'config Config) -> Self {
        Self::new_with_precompile(backend, gas_limit, config, precompiles)
    }

    /// Create a new stack-based executor with given precompiles.
    pub fn new_with_precompile(
        backend: &'backend B,
        gas_limit: usize,
        config: &'config Config,
        precompiles: fn(
            H160,
            &[u8],
            Option<usize>,
        ) -> Option<Result<(ExitSucceed, Vec<u8>, usize), ExitError>>,
    ) -> Self {
        Self {
            backend,
            gasometer: Gasometer::new(gas_limit, config),
            state: BTreeMap::new(),
            deleted: BTreeSet::new(),
            config,
            logs: Vec::new(),
            precompile: precompiles,
            is_static: false,
            depth: None,
            pending_txs: Vec::new(),
            tmp_bn: None,
            tmp_timestamp: None,
            created_contracts: BTreeSet::new(),
            call_trace: Vec::new(),
            precompiles: BTreeMap::new(),
        }
    }

    /// Create a substate executor from the current executor.
    pub fn substate(
        &self,
        gas_limit: usize,
        is_static: bool,
    ) -> StackExecutor<'backend, 'config, B> {
        Self {
            backend: self.backend,
            gasometer: Gasometer::new(gas_limit, self.gasometer.config()),
            config: self.config,
            state: self.state.clone(),
            deleted: self.deleted.clone(),
            logs: Vec::new(),
            precompile: self.precompile,
            is_static: is_static || self.is_static,
            depth: match self.depth {
                None => Some(0),
                Some(n) => Some(n + 1),
            },
            pending_txs: Vec::new(),
            tmp_bn: self.tmp_bn,
            tmp_timestamp: self.tmp_timestamp,
            created_contracts: self.created_contracts.clone(),
            call_trace: Vec::new(),
            precompiles: self.precompiles.clone(),
        }
    }

    /// Execute the runtime until it returns.
    pub fn execute(&mut self, runtime: &mut Runtime) -> ExitReason {
        match runtime.run(self) {
            Capture::Exit(s) => s,
            Capture::Trap(_) => unreachable!("Trap is Infallible"),
        }
    }

    /// Get remaining gas.
    pub fn gas(&self) -> usize {
        self.gasometer.gas()
    }

    /// Merge a substate executor that succeeded.
    pub fn merge_succeed<'obackend, 'oconfig, OB>(
        &mut self,
        mut substate: StackExecutor<'obackend, 'oconfig, OB>,
        mut calltrace: CallTrace,
    ) -> Result<(), ExitError> {
        calltrace.logs = substate.logs.clone();
        self.call_trace.push(Box::new(calltrace));
        self.logs.append(&mut substate.logs);
        self.deleted.append(&mut substate.deleted);
        for cc in substate.created_contracts.into_iter() {
            self.created_contracts.insert(cc.clone());
        }
        self.state = substate.state;
        self.tmp_bn = substate.tmp_bn;
        self.tmp_timestamp = substate.tmp_timestamp;
        self.gasometer.record_stipend(substate.gasometer.gas())?;
        self.gasometer
            .record_refund(substate.gasometer.refunded_gas())?;
        Ok(())
    }

    /// Merge a substate executor that reverted.
    pub fn merge_revert<'obackend, 'oconfig, OB>(
        &mut self,
        mut substate: StackExecutor<'obackend, 'oconfig, OB>,
        mut calltrace: CallTrace,
    ) -> Result<(), ExitError> {
        calltrace.logs = substate.logs.clone();
        self.call_trace.push(Box::new(calltrace));
        self.logs.append(&mut substate.logs);
        self.tmp_bn = substate.tmp_bn;
        self.tmp_timestamp = substate.tmp_timestamp;
        self.gasometer.record_stipend(substate.gasometer.gas())?;
        Ok(())
    }

    /// Merge a substate executor that failed.
    pub fn merge_fail<'obackend, 'oconfig, OB>(
        &mut self,
        mut substate: StackExecutor<'obackend, 'oconfig, OB>,
        mut calltrace: CallTrace,
    ) -> Result<(), ExitError> {
        calltrace.logs = substate.logs.clone();
        self.call_trace.push(Box::new(calltrace));
        self.tmp_bn = substate.tmp_bn;
        self.tmp_timestamp = substate.tmp_timestamp;
        self.logs.append(&mut substate.logs);

        Ok(())
    }

    /// Execute a `CREATE` transaction.
    pub fn transact_create(
        &mut self,
        hash: H256,
        caller: H160,
        value: U256,
        init_code: Vec<u8>,
        gas_limit: usize,
    ) -> (ExitReason, Option<H160>, Vec<Box<CallTrace>>) {
        let transaction_cost = gasometer::create_transaction_cost(&init_code);
        match self.gasometer.record_transaction(transaction_cost) {
            Ok(()) => (),
            Err(e) => return (e.into(), None, Vec::new()),
        }

        let exit = self.create_inner(
            caller,
            CreateScheme::Legacy { caller },
            value,
            init_code,
            Some(gas_limit),
            false,
        );

        let status;
        match exit {
            Capture::Exit((s, _a, _)) => match s {
                ExitReason::Succeed(_) => {
                    status = 1;
                }
                _ => {
                    status = 0;
                }
            },
            Capture::Trap(_) => unreachable!(),
        };

        self.pending_txs.push(TxReceipt {
            hash,
            caller,
            to: None,
            block_number: self.backend.block_number(),
            cumulative_gas_used: self.used_gas(),
            gas_used: self.used_gas(),
            contract_addresses: self.created_contracts.clone(),
            logs: self.logs.clone(),
            status,
        });

        self.tmp_bn = None;
        self.tmp_timestamp = None;

        match exit {
            Capture::Exit((s, a, _)) => (s, a, self.call_trace.clone()),
            Capture::Trap(_) => unreachable!(),
        }
    }

    /// Execute a `CREATE2` transaction.
    pub fn transact_create2(
        &mut self,
        caller: H160,
        value: U256,
        init_code: Vec<u8>,
        salt: H256,
        gas_limit: usize,
    ) -> (ExitReason, Option<H160>, Vec<Box<CallTrace>>) {
        let transaction_cost = gasometer::create_transaction_cost(&init_code);
        match self.gasometer.record_transaction(transaction_cost) {
            Ok(()) => (),
            Err(e) => return (e.into(), None, Vec::new()),
        }
        let code_hash = H256::from_slice(Keccak256::digest(&init_code).as_slice());

        let exit = self.create_inner(
            caller,
            CreateScheme::Create2 {
                caller,
                code_hash,
                salt,
            },
            value,
            init_code,
            Some(gas_limit),
            false,
        );

        // self.call_trace = self.call_trace.inner.clone();

        match exit {
            Capture::Exit((s, a, _)) => (s, a, self.call_trace.clone()),
            Capture::Trap(_) => unreachable!(),
        }
    }

    /// Execute a `CALL` transaction.
    pub fn transact_call(
        &mut self,
        hash: H256,
        caller: H160,
        address: H160,
        value: U256,
        data: Vec<u8>,
        gas_limit: usize,
    ) -> (ExitReason, Vec<u8>, Vec<Box<CallTrace>>) {
        let transaction_cost = gasometer::call_transaction_cost(&data);
        match self.gasometer.record_transaction(transaction_cost) {
            Ok(()) => (),
            Err(e) => return (e.into(), Vec::new(), Vec::new()),
        }

        self.account_mut(caller).basic.nonce += U256::one();

        let context = Context {
            caller,
            address,
            apparent_value: value,
        };

        let exit = self.call_inner(
            address,
            Some(Transfer {
                source: caller,
                target: address,
                value,
            }),
            data,
            Some(gas_limit),
            false,
            false,
            false,
            context,
        );

        // self.call_trace = self.call_trace.inner.clone();

        let status;
        match exit {
            Capture::Exit((s, ref _v)) => match s {
                ExitReason::Succeed(_) => {
                    status = 1;
                }
                _ => {
                    status = 0;
                }
            },
            Capture::Trap(_) => unreachable!(),
        };

        self.pending_txs.push(TxReceipt {
            hash,
            caller,
            to: Some(address),
            block_number: self.backend.block_number(),
            cumulative_gas_used: self.used_gas(),
            gas_used: self.used_gas(),
            contract_addresses: self.created_contracts.clone(),
            logs: self.logs.clone(),
            status,
        });

        self.tmp_bn = None;
        self.tmp_timestamp = None;

        match exit {
            Capture::Exit((s, v)) => (s, v, self.call_trace.clone()),
            Capture::Trap(_) => unreachable!(),
        }
    }

    /// Get used gas for the current executor, given the price.
    pub fn used_gas(&self) -> usize {
        self.gasometer.total_used_gas()
            - min(
                self.gasometer.total_used_gas() / 2,
                self.gasometer.refunded_gas() as usize,
            )
    }

    /// Get fee needed for the current executor, given the price.
    pub fn fee(&self, price: U256) -> U256 {
        let used_gas = self.used_gas();
        U256::from(used_gas) * price
    }

    /// Deconstruct the executor, return state to be applied.
    #[must_use]
    pub fn deconstruct(
        self,
    ) -> (
        impl IntoIterator<Item = Apply<impl IntoIterator<Item = (H256, H256)>>>,
        impl IntoIterator<Item = Log>,
        Vec<TxReceipt>,
        BTreeSet<H160>,
    ) {
        let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

        for (address, account) in self.state {
            if self.deleted.contains(&address) {
                continue;
            }
            let mut storage = BTreeMap::new();
            if account.reset_storage_backend {
                for (slot, _val) in account.storage.iter() {
                    if let Some(strg) = account.original_storage.get(&slot) {
                        storage.insert(slot.clone(), strg.clone());
                    } else {
                        let strg = self.backend.storage(address.clone(), slot.clone());
                        storage.insert(slot.clone(), strg.clone());
                    }
                }
            } else {
                storage = account.storage.clone();
            }

            applies.push(Apply::Modify {
                address,
                basic: account.basic,
                code: account.code,
                storage,
                reset_storage: account.reset_storage,
            });
        }

        for address in self.deleted {
            applies.push(Apply::Delete { address });
        }

        let logs = self.logs;
        let txs = self.pending_txs;

        (applies, logs, txs, self.created_contracts.clone())
    }

    /// Deconstruct the executor, return state to be applied.
    #[must_use]
    pub fn deconstruct_fork_only(
        self,
    ) -> (
        impl IntoIterator<Item = Apply<impl IntoIterator<Item = (H256, H256)>>>,
        impl IntoIterator<Item = Log>,
        Vec<TxReceipt>,
        BTreeSet<H160>,
    ) {
        let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

        for (address, account) in self.state {
            if !self.created_contracts.contains(&address) {
                let storage = account.original_storage.clone();
                let basic = account.original_basic.clone();
                let code = account.original_code.clone();

                applies.push(Apply::Modify {
                    address,
                    basic,
                    code,
                    storage,
                    reset_storage: account.reset_storage,
                });
            }
        }

        for address in self.deleted {
            applies.push(Apply::Delete { address });
        }

        let logs = self.logs;
        let txs = self.pending_txs;

        (applies, logs, txs, self.created_contracts.clone())
    }

    /// Get mutable account reference.
    pub fn account_mut(&mut self, address: H160) -> &mut StackAccount {
        if self.state.contains_key(&address) {
            self.state.get_mut(&address).unwrap()
        } else {
            let b = self.backend.basic(address);
            let acct = StackAccount {
                basic: b.clone(),
                code: None,
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: None,
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            self.state.insert(address, acct);
            self.state.get_mut(&address).unwrap()
        }
    }

    /// Get account nonce.
    pub fn nonce(&mut self, address: H160) -> U256 {
        if self.state.contains_key(&address) {
            self.state.get_mut(&address).unwrap().basic.nonce
        } else {
            let b = self.backend.basic(address);
            let acct = StackAccount {
                basic: b.clone(),
                code: None,
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: None,
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            self.state.insert(address, acct);
            self.state.get_mut(&address).unwrap().basic.nonce
        }
    }

    /// Withdraw balance from address.
    pub fn withdraw(&mut self, address: H160, balance: U256) -> Result<(), ExitError> {
        let source = self.account_mut(address);
        if source.basic.balance < balance {
            return Err(ExitError::OutOfFund.into());
        }
        source.basic.balance -= balance;

        Ok(())
    }

    /// Deposit balance to address.
    pub fn deposit(&mut self, address: H160, balance: U256) {
        let target = self.account_mut(address);
        target.basic.balance += balance;
    }

    /// Transfer balance with the given struct.
    pub fn transfer(&mut self, transfer: Transfer) -> Result<(), ExitError> {
        self.withdraw(transfer.source, transfer.value)?;
        self.deposit(transfer.target, transfer.value);

        Ok(())
    }

    /// Get the create address from given scheme.
    pub fn create_address(&mut self, scheme: CreateScheme) -> H160 {
        match scheme {
            CreateScheme::Create2 {
                caller,
                code_hash,
                salt,
            } => {
                let mut hasher = Keccak256::new();
                hasher.input(&[0xff]);
                hasher.input(&caller[..]);
                hasher.input(&salt[..]);
                hasher.input(&code_hash[..]);
                H256::from_slice(hasher.result().as_slice()).into()
            }
            CreateScheme::Legacy { caller } => {
                let nonce = self.nonce(caller);
                let mut stream = rlp::RlpStream::new_list(2);
                stream.append(&caller);
                stream.append(&nonce);
                H256::from_slice(Keccak256::digest(&stream.out()).as_slice()).into()
            }
            CreateScheme::Fixed(naddress) => naddress,
        }
    }

    fn create_inner(
        &mut self,
        caller: H160,
        scheme: CreateScheme,
        value: U256,
        init_code: Vec<u8>,
        target_gas: Option<usize>,
        take_l64: bool,
    ) -> Capture<(ExitReason, Option<H160>, Vec<u8>), Infallible> {
        let mut calltrace = CallTrace::default();
        macro_rules! try_or_fail {
            ( $e:expr ) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Capture::Exit((e.into(), None, Vec::new())),
                }
            };
        }

        fn l64(gas: usize) -> usize {
            gas - gas / 64
        }

        if let Some(depth) = self.depth {
            if depth + 1 > self.config.call_stack_limit {
                return Capture::Exit((ExitError::CallTooDeep.into(), None, Vec::new()));
            }
        }

        if self.balance(caller) < value {
            return Capture::Exit((ExitError::OutOfFund.into(), None, Vec::new()));
        }

        let mut after_gas = self.gasometer.gas();

        if take_l64 && self.config.call_l64_after_gas {
            after_gas = l64(after_gas);
        }
        let target_gas = target_gas.unwrap_or(after_gas);

        let gas_limit = min(after_gas, target_gas);
        try_or_fail!(self.gasometer.record_cost(gas_limit));

        let address = self.create_address(scheme);

        self.created_contracts.insert(address);

        println!("Created address: {:?}", address);

        self.account_mut(caller).basic.nonce += U256::one();

        let mut substate = self.substate(gas_limit, false);
        {
            // already exists
            if let Some(code) = substate.account_mut(address).code.as_ref() {
                if code.len() != 0 {
                    calltrace.success = false;
                    calltrace.addr = address;
                    calltrace.created = true;
                    calltrace.cost = substate.used_gas();
                    calltrace.input = hex::encode(init_code);
                    calltrace.inner.append(&mut substate.call_trace);
                    let _ = self.merge_fail(substate, calltrace);
                    return Capture::Exit((ExitError::CreateCollision.into(), None, Vec::new()));
                }
            } else {
                // is a real contract
                let code = substate.backend.code(address);
                substate.account_mut(address).code = Some(code.clone());

                if code.len() != 0 {
                    calltrace.success = false;
                    calltrace.addr = address;
                    calltrace.created = true;
                    calltrace.cost = substate.used_gas();
                    calltrace.input = hex::encode(init_code);
                    calltrace.inner.append(&mut substate.call_trace);
                    let _ = self.merge_fail(substate, calltrace);
                    return Capture::Exit((ExitError::CreateCollision.into(), None, Vec::new()));
                }
            }
            // is a wallet
            if substate.account_mut(address).basic.nonce > U256::zero() {
                calltrace.success = false;
                calltrace.addr = address;
                calltrace.created = true;
                calltrace.cost = substate.used_gas();
                calltrace.input = hex::encode(init_code);
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_fail(substate, calltrace);
                return Capture::Exit((ExitError::CreateCollision.into(), None, Vec::new()));
            }

            substate.account_mut(address).reset_storage = true;
            substate.account_mut(address).storage = BTreeMap::new();
        }

        let context = Context {
            address,
            caller,
            apparent_value: value,
        };
        let transfer = Transfer {
            source: caller,
            target: address,
            value,
        };
        match substate.transfer(transfer) {
            Ok(()) => (),
            Err(e) => {
                calltrace.success = false;
                calltrace.addr = address;
                calltrace.created = true;
                calltrace.cost = substate.used_gas();
                calltrace.input = hex::encode(init_code);
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_revert(substate, calltrace);
                return Capture::Exit((ExitReason::Error(e), None, Vec::new()));
            }
        }

        if self.config.create_increase_nonce {
            substate.account_mut(address).basic.nonce += U256::one();
        }

        let mut runtime = Runtime::new(
            Rc::new(init_code.clone()),
            Rc::new(Vec::new()),
            context,
            self.config,
        );

        let reason = substate.execute(&mut runtime);

        match reason {
            ExitReason::Succeed(s) => {
                let out = runtime.machine().return_value();

                if let Some(limit) = self.config.create_contract_limit {
                    if out.len() > limit {
                        calltrace.success = false;
                        calltrace.addr = address;
                        calltrace.created = true;
                        substate.gasometer.fail();
                        calltrace.cost = substate.used_gas();
                        calltrace.input = hex::encode(init_code);
                        calltrace.output = hex::encode(runtime.machine.return_value());
                        calltrace.inner.append(&mut substate.call_trace);
                        let _ = self.merge_fail(substate, calltrace);
                        return Capture::Exit((
                            ExitError::CreateContractLimit.into(),
                            None,
                            Vec::new(),
                        ));
                    }
                }

                match substate.gasometer.record_deposit(out.len()) {
                    Ok(()) => {
                        calltrace.success = true;
                        calltrace.addr = address;
                        calltrace.created = true;
                        calltrace.cost = substate.used_gas();
                        calltrace.input = hex::encode(init_code);
                        calltrace.output = hex::encode(runtime.machine.return_value());
                        calltrace.inner.append(&mut substate.call_trace);
                        let e = self.merge_succeed(substate, calltrace);

                        if self.state.contains_key(&address) {
                            let acct = self.state.get_mut(&address).unwrap();
                            acct.code = Some(out);
                            acct.original_code = acct.code.clone();
                            *acct = acct.clone();
                        } else {
                            let b = self.backend.basic(address);
                            let acct = StackAccount {
                                basic: b.clone(),
                                code: Some(out.clone()),
                                storage: BTreeMap::new(),
                                original_storage: BTreeMap::new(),
                                original_code: Some(out),
                                original_basic: b,
                                reset_storage: false,
                                reset_storage_backend: false,
                            };
                            self.state.insert(address, acct.clone());
                        }
                        try_or_fail!(e);
                        Capture::Exit((ExitReason::Succeed(s), Some(address), Vec::new()))
                    }
                    Err(e) => {
                        calltrace.success = false;
                        calltrace.addr = address;
                        calltrace.created = true;
                        calltrace.cost = substate.used_gas();
                        calltrace.input = hex::encode(init_code);
                        calltrace.output = hex::encode(runtime.machine.return_value());
                        calltrace.inner.append(&mut substate.call_trace);
                        let _ = self.merge_fail(substate, calltrace);
                        Capture::Exit((ExitReason::Error(e), None, Vec::new()))
                    }
                }
            }
            ExitReason::Error(e) => {
                calltrace.success = false;
                calltrace.addr = address;
                calltrace.created = true;
                substate.gasometer.fail();
                calltrace.cost = substate.used_gas();
                calltrace.input = hex::encode(init_code);
                calltrace.output = hex::encode(runtime.machine.return_value());
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_fail(substate, calltrace);
                Capture::Exit((ExitReason::Error(e), None, Vec::new()))
            }
            ExitReason::Revert(e) => {
                calltrace.success = false;
                calltrace.addr = address;
                calltrace.created = true;
                calltrace.cost = substate.used_gas();
                calltrace.input = hex::encode(init_code);
                calltrace.output = hex::encode(runtime.machine.return_value());
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_revert(substate, calltrace);
                Capture::Exit((
                    ExitReason::Revert(e),
                    None,
                    runtime.machine().return_value(),
                ))
            }
            ExitReason::Fatal(e) => {
                self.gasometer.fail();
                Capture::Exit((ExitReason::Fatal(e), None, Vec::new()))
            }
        }
    }

    fn call_inner(
        &mut self,
        code_address: H160,
        transfer: Option<Transfer>,
        input: Vec<u8>,
        target_gas: Option<usize>,
        is_static: bool,
        take_l64: bool,
        take_stipend: bool,
        context: Context,
    ) -> Capture<(ExitReason, Vec<u8>), Infallible> {
        let mut calltrace = CallTrace::default();

        let mut forced_ret = H256::zero();
        let mut is_forced_ret = false;
        if code_address == "7109709ECfa91a80626fF3989D68f67F5b1DD12D".parse().unwrap() {
            let sig = hex::encode([input[0], input[1], input[2], input[3]]);
            match sig {
                // roll
                _ if sig == "1f7b4f30".to_string() => {
                    let amount = U256::from_big_endian(&input[4..]);
                    self.tmp_bn = Some(amount);
                }
                // warp
                _ if sig == "e5d6bf02".to_string() => {
                    let timestamp = U256::from_big_endian(&input[4..]);
                    self.tmp_timestamp = Some(timestamp);
                }
                // store
                _ if sig == "70ca10bb".to_string() => {
                    let who = H160::from_slice(&input[16..36]);
                    let slot = H256::from_slice(&input[36..68]);
                    let val = H256::from_slice(&input[68..]);
                    if self.state.contains_key(&who) {
                        let acct = self.state.get_mut(&who).unwrap();
                        acct.storage.insert(slot, val);
                        acct.reset_storage_backend = false;
                        *acct = acct.clone();
                    } else {
                        let b = self.backend.basic(who);
                        let code = self.backend.code(who);
                        let mut acct = StackAccount {
                            basic: b.clone(),
                            code: Some(code.clone()),
                            storage: BTreeMap::new(),
                            original_storage: BTreeMap::new(),
                            original_code: Some(code),
                            original_basic: b,
                            reset_storage: false,
                            reset_storage_backend: false,
                        };
                        acct.storage.insert(slot, val);
                        self.state.insert(who, acct);
                    }
                }
                // load
                _ if sig == "667f9d70".to_string() => {
                    let who = H160::from_slice(&input[16..36]);
                    let slot = H256::from_slice(&input[36..68]);
                    forced_ret = self.storage(who, slot);
                    is_forced_ret = true;
                }
                _ => {}
            }
        }

        macro_rules! try_or_fail {
            ( $e:expr ) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Capture::Exit((e.into(), Vec::new())),
                }
            };
        }

        fn l64(gas: usize) -> usize {
            gas - gas / 64
        }

        let mut after_gas = self.gasometer.gas();
        if take_l64 && self.config.call_l64_after_gas {
            after_gas = l64(after_gas);
        }

        let target_gas = target_gas.unwrap_or(after_gas);
        let mut gas_limit = min(target_gas, after_gas);

        try_or_fail!(self.gasometer.record_cost(gas_limit));

        if let Some(transfer) = transfer.as_ref() {
            if take_stipend && transfer.value != U256::zero() {
                gas_limit = gas_limit.saturating_add(self.config.call_stipend);
            }
        }

        let code = self.code(code_address);

        let mut substate = self.substate(gas_limit, is_static);
        substate.account_mut(context.address);

        let mut sig: [u8; 4] = Default::default();
        let i;
        if input.len() >= 4 {
            sig.copy_from_slice(&input.clone()[..4]);
        }

        if input.len() > 4 {
            i = hex::encode(&input[4..]);
        } else {
            i = hex::encode(Vec::new());
        }

        if let Some(depth) = self.depth {
            if depth + 1 > self.config.call_stack_limit {
                calltrace.success = false;
                calltrace.addr = code_address;
                calltrace.created = false;
                calltrace.function = hex::encode(sig);
                calltrace.input = i;
                calltrace.cost = substate.used_gas();
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_revert(substate, calltrace);
                return Capture::Exit((ExitError::CallTooDeep.into(), Vec::new()));
            }
        }

        if let Some(transfer) = transfer {
            match substate.transfer(transfer) {
                Ok(()) => (),
                Err(e) => {
                    calltrace.success = false;
                    calltrace.addr = code_address;
                    calltrace.created = false;
                    calltrace.function = hex::encode(sig);
                    calltrace.input = i;
                    calltrace.cost = substate.used_gas();
                    calltrace.inner.append(&mut substate.call_trace);
                    let _ = self.merge_revert(substate, calltrace);
                    return Capture::Exit((ExitReason::Error(e), Vec::new()));
                }
            }
        }

        if let Some(ret) = (substate.precompile)(code_address, &input, Some(gas_limit)) {
            return match ret {
                Ok((s, out, cost)) => {
                    calltrace.success = true;
                    calltrace.addr = code_address;
                    calltrace.created = false;
                    calltrace.function = hex::encode(sig);
                    calltrace.input = i;
                    calltrace.cost = substate.used_gas();
                    calltrace.inner.append(&mut substate.call_trace);
                    let _ = substate.gasometer.record_cost(cost);
                    let _ = self.merge_succeed(substate, calltrace);
                    Capture::Exit((ExitReason::Succeed(s), out))
                }
                Err(e) => {
                    calltrace.success = false;
                    calltrace.addr = code_address;
                    calltrace.created = false;
                    calltrace.function = hex::encode(sig);
                    calltrace.input = i;
                    calltrace.cost = substate.used_gas();
                    calltrace.inner.append(&mut substate.call_trace);
                    let _ = self.merge_fail(substate, calltrace);
                    Capture::Exit((ExitReason::Error(e), Vec::new()))
                }
            };
        }

        let mut runtime = Runtime::new(Rc::new(code), Rc::new(input.clone()), context, self.config);

        if code_address == "7109709ECfa91a80626fF3989D68f67F5b1DD12D".parse().unwrap()
            && is_forced_ret
        {
            runtime.machine.return_range = U256::zero()..U256::from(32);
            runtime
                .machine
                .memory
                .set(0, forced_ret.as_bytes(), None)
                .unwrap();
        }

        let reason = substate.execute(&mut runtime);

        match reason {
            ExitReason::Succeed(s) => {
                calltrace.success = true;
                calltrace.addr = code_address;
                calltrace.created = false;
                calltrace.function = hex::encode(sig);
                calltrace.input = i;
                calltrace.cost = substate.used_gas();
                calltrace.output = hex::encode(runtime.machine.return_value());
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_succeed(substate, calltrace);
                Capture::Exit((ExitReason::Succeed(s), runtime.machine().return_value()))
            }
            ExitReason::Error(e) => {
                calltrace.success = false;
                calltrace.addr = code_address;
                calltrace.created = false;
                calltrace.function = hex::encode(sig);
                calltrace.input = i;
                calltrace.cost = substate.used_gas();
                calltrace.output = hex::encode(runtime.machine.return_value());
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_fail(substate, calltrace);
                Capture::Exit((ExitReason::Error(e), Vec::new()))
            }
            ExitReason::Revert(e) => {
                calltrace.success = false;
                calltrace.addr = code_address;
                calltrace.created = false;
                calltrace.function = hex::encode(sig);
                calltrace.input = i;
                calltrace.cost = substate.used_gas();
                calltrace.output = hex::encode(runtime.machine.return_value());
                calltrace.inner.append(&mut substate.call_trace);
                let _ = self.merge_revert(substate, calltrace);
                Capture::Exit((ExitReason::Revert(e), runtime.machine().return_value()))
            }
            ExitReason::Fatal(e) => {
                self.gasometer.fail();
                Capture::Exit((ExitReason::Fatal(e), Vec::new()))
            }
        }
    }
}

impl<'backend, 'config, B: Backend> Handler for StackExecutor<'backend, 'config, B> {
    type CreateInterrupt = Infallible;
    type CreateFeedback = Infallible;
    type CallInterrupt = Infallible;
    type CallFeedback = Infallible;

    fn balance(&mut self, address: H160) -> U256 {
        if self.state.contains_key(&address) {
            self.state.get_mut(&address).unwrap().basic.balance
        } else {
            let b = self.backend.basic(address);
            let acct = StackAccount {
                basic: b.clone(),
                code: None,
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: None,
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            self.state.insert(address, acct);
            self.state.get_mut(&address).unwrap().basic.balance
        }
    }

    fn code_size(&mut self, address: H160) -> U256 {
        if address == "7109709ECfa91a80626fF3989D68f67F5b1DD12D".parse().unwrap() {
            return U256::from(100);
        }
        if self.state.contains_key(&address) {
            let acct = self.state.get_mut(&address).unwrap();
            if let Some(c) = acct.code.clone() {
                U256::from(c.len())
            } else {
                acct.code = Some(self.backend.code(address));
                acct.original_code = acct.code.clone();
                U256::from(acct.code.clone().unwrap().len())
            }
        } else {
            let b = self.backend.basic(address);
            let code = self.backend.code(address);
            let acct = StackAccount {
                basic: b.clone(),
                code: Some(code.clone()),
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: Some(code),
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            self.state.insert(address, acct);
            U256::from(
                self.state
                    .get_mut(&address)
                    .unwrap()
                    .code
                    .clone()
                    .unwrap()
                    .len(),
            )
        }
    }

    fn code_hash(&mut self, address: H160) -> H256 {
        if self.state.contains_key(&address) {
            let acct = self.state.get_mut(&address).unwrap();
            if let Some(c) = acct.code.clone() {
                H256::from_slice(Keccak256::digest(&c).as_slice())
            } else {
                acct.code = Some(self.backend.code(address));
                H256::from_slice(Keccak256::digest(&acct.code.clone().unwrap()).as_slice())
            }
        } else {
            let b = self.backend.basic(address);
            let code = self.backend.code(address);
            let acct = StackAccount {
                basic: b.clone(),
                code: Some(code.clone()),
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: Some(code),
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            self.state.insert(address, acct.clone());
            H256::from_slice(Keccak256::digest(&acct.code.clone().unwrap()).as_slice())
        }
    }

    fn code(&mut self, address: H160) -> Vec<u8> {
        if self.state.contains_key(&address) {
            let acct = self.state.get_mut(&address).unwrap();
            if let Some(c) = acct.code.clone() {
                c
            } else {
                acct.code = Some(self.backend.code(address));
                acct.original_code = acct.code.clone();
                acct.code.clone().unwrap()
            }
        } else {
            let b = self.backend.basic(address);
            let code = self.backend.code(address);
            let acct = StackAccount {
                basic: b.clone(),
                code: Some(code.clone()),
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: Some(code),
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            self.state.insert(address, acct.clone());
            acct.code.clone().unwrap()
        }
    }

    fn storage(&mut self, address: H160, index: H256) -> H256 {
        if self.state.contains_key(&address) {
            let acct = self.state.get_mut(&address).unwrap();
            if let Some(storage_data) = acct.storage.get(&index) {
                storage_data.clone()
            } else if let Some(storage_data) = acct.original_storage.get(&index) {
                storage_data.clone()
            } else if self.created_contracts.contains(&address) {
                // this contract was created by self, dont call backend for it
                H256::default()
            } else {
                let storage_data = self.backend.storage(address, index);
                acct.storage.insert(index, storage_data);
                acct.original_storage.insert(index, storage_data);
                *acct = acct.clone();
                storage_data
            }
        } else {
            let b = self.backend.basic(address);
            let code = self.backend.code(address);
            let mut acct = StackAccount {
                basic: b.clone(),
                code: Some(code.clone()),
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: Some(code),
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            let storage_data = self.backend.storage(address, index);
            acct.storage.insert(index, storage_data);
            acct.original_storage.insert(index, storage_data);
            self.state.insert(address, acct);
            storage_data
        }
    }

    fn original_storage(&mut self, address: H160, index: H256) -> H256 {
        if let Some(account) = self.state.get_mut(&address) {
            if account.reset_storage {
                return H256::default();
            } else if let Some(strg) = account.original_storage.get(&index) {
                return strg.clone();
            } else {
                let storage = self.backend.storage(address, index);
                account.original_storage.insert(index, storage);
                *account = account.clone();
                return storage;
            }
        }
        if self.created_contracts.contains(&address) {
            // this contract was created by self (this tx), dont call backend for it
            H256::default()
        } else {
            let b = self.backend.basic(address);
            let code = self.backend.code(address);
            let mut acct = StackAccount {
                basic: b.clone(),
                code: Some(code.clone()),
                storage: BTreeMap::new(),
                original_storage: BTreeMap::new(),
                original_code: Some(code),
                original_basic: b,
                reset_storage: false,
                reset_storage_backend: false,
            };
            let storage = self.backend.storage(address, index);
            acct.original_storage.insert(index, storage);
            self.state.insert(address, acct);
            storage
        }
    }

    fn exists(&self, address: H160) -> bool {
        if self.config.empty_considered_exists {
            self.state.get(&address).is_some() || self.backend.exists(address)
        } else if let Some(account) = self.state.get(&address) {
            account.basic.nonce != U256::zero()
                || account.basic.balance != U256::zero()
                || account.code.as_ref().map(|c| c.len() != 0).unwrap_or(false)
                || self.backend.code(address).len() != 0
        } else {
            self.backend.basic(address).nonce != U256::zero()
                || self.backend.basic(address).balance != U256::zero()
                || self.backend.code(address).len() != 0
        }
    }

    fn gas_left(&self) -> U256 {
        U256::from(self.gasometer.gas())
    }

    fn gas_price(&self) -> U256 {
        self.backend.gas_price()
    }
    fn origin(&self) -> H160 {
        self.backend.origin()
    }
    fn block_hash(&self, number: U256) -> H256 {
        self.backend.block_hash(number)
    }
    fn block_number(&self) -> U256 {
        if let Some(tmpbn) = self.tmp_bn {
            tmpbn
        } else {
            self.backend.block_number()
        }
    }
    fn block_coinbase(&self) -> H160 {
        self.backend.block_coinbase()
    }
    fn block_timestamp(&self) -> U256 {
        if let Some(tmptm) = self.tmp_timestamp {
            tmptm
        } else {
            self.backend.block_timestamp()
        }
    }
    fn block_difficulty(&self) -> U256 {
        self.backend.block_difficulty()
    }
    fn block_gas_limit(&self) -> U256 {
        self.backend.block_gas_limit()
    }
    fn chain_id(&self) -> U256 {
        self.backend.chain_id()
    }

    fn deleted(&self, address: H160) -> bool {
        self.deleted.contains(&address)
    }

    fn set_storage(&mut self, address: H160, index: H256, value: H256) -> Result<(), ExitError> {
        self.account_mut(address).storage.insert(index, value);

        Ok(())
    }

    fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) -> Result<(), ExitError> {
        self.logs.push(Log {
            address,
            topics,
            data,
        });

        Ok(())
    }

    fn mark_delete(&mut self, address: H160, target: H160) -> Result<(), ExitError> {
        let balance = self.balance(address);

        self.transfer(Transfer {
            source: address,
            target,
            value: balance,
        })?;
        self.account_mut(address).basic.balance = U256::zero();

        self.deleted.insert(address);

        Ok(())
    }

    fn create(
        &mut self,
        caller: H160,
        scheme: CreateScheme,
        value: U256,
        init_code: Vec<u8>,
        target_gas: Option<usize>,
    ) -> Capture<(ExitReason, Option<H160>, Vec<u8>), Self::CreateInterrupt> {
        self.create_inner(caller, scheme, value, init_code, target_gas, true)
    }

    fn call(
        &mut self,
        code_address: H160,
        transfer: Option<Transfer>,
        input: Vec<u8>,
        target_gas: Option<usize>,
        is_static: bool,
        context: Context,
    ) -> Capture<(ExitReason, Vec<u8>), Self::CallInterrupt> {
        self.call_inner(
            code_address,
            transfer,
            input,
            target_gas,
            is_static,
            true,
            true,
            context,
        )
    }

    fn pre_validate(
        &mut self,
        context: &Context,
        opcode: Result<Opcode, ExternalOpcode>,
        stack: &Stack,
    ) -> Result<(), ExitError> {
        let (gas_cost, memory_cost) = gasometer::opcode_cost(
            context.address,
            opcode,
            stack,
            self.is_static,
            &self.config,
            self,
        )?;
        self.gasometer.record_opcode(gas_cost, memory_cost)?;

        Ok(())
    }
}
