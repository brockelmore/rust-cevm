//! Runtime layer for EVM.

#![deny(warnings)]
#![forbid(unsafe_code, missing_docs, unused_variables, unused_imports)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod context;
mod eval;
mod handler;
mod interrupt;

pub use evm_core::*;

pub use crate::context::{CallScheme, Context, CreateScheme};
pub use crate::handler::{Handler, Transfer};
pub use crate::interrupt::{Resolve, ResolveCall, ResolveCreate};

use alloc::rc::Rc;
use alloc::vec::Vec;

macro_rules! step {
	( $self:expr, $handler:expr, $return:tt $($err:path)?; $($ok:path)? ) => ({
		if let Some((opcode, stack)) = $self.machine.inspect() {
			match $handler.pre_validate(&$self.context, opcode, stack) {
				Ok(()) => (),
				Err(e) => {
					$self.machine.exit(e.into());
					$self.status = Err(e.into());
				},
			}
		}

		match $self.status.clone() {
			Ok(()) => (),
			Err(e) => {
				#[allow(unused_parens)]
				$return $($err)*(Capture::Exit(e))
			},
		}

		match $self.machine.step() {
			Ok(()) => $($ok)?(()),
			Err(Capture::Exit(e)) => {
				$self.status = Err(e);
				#[allow(unused_parens)]
				$return $($err)*(Capture::Exit(e))
			},
			Err(Capture::Trap(opcode)) => {
				match eval::eval($self, opcode, $handler) {
					eval::Control::Continue => $($ok)?(()),
					eval::Control::CallInterrupt(interrupt) => {
						let resolve = ResolveCall::new($self);
						#[allow(unused_parens)]
						$return $($err)*(Capture::Trap(Resolve::Call(interrupt, resolve)))
					},
					eval::Control::CreateInterrupt(interrupt) => {
						let resolve = ResolveCreate::new($self);
						#[allow(unused_parens)]
						$return $($err)*(Capture::Trap(Resolve::Create(interrupt, resolve)))
					},
					eval::Control::Exit(exit) => {
						$self.machine.exit(exit.into());
						$self.status = Err(exit);
						#[allow(unused_parens)]
						$return $($err)*(Capture::Exit(exit))
					},
				}
			},
		}
	});
}

/// EVM runtime.
///
/// The runtime wraps an EVM `Machine` with support of return data and context.
pub struct Runtime<'config> {
    /// stack machine
    pub machine: Machine,
    status: Result<(), ExitReason>,
    /// return data buffer
    pub return_data_buffer: Vec<u8>,
    context: Context,
    _config: &'config Config,
}

impl<'config> Runtime<'config> {
    /// Create a new runtime with given code and data.
    pub fn new(
        code: Rc<Vec<u8>>,
        data: Rc<Vec<u8>>,
        context: Context,
        config: &'config Config,
    ) -> Self {
        Self {
            machine: Machine::new(code, data, config.stack_limit, config.memory_limit),
            status: Ok(()),
            return_data_buffer: Vec::new(),
            context,
            _config: config,
        }
    }

    /// Get a reference to the machine.
    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    /// Step the runtime.
    pub fn step<'a, H: Handler>(
        &'a mut self,
        handler: &mut H,
    ) -> Result<(), Capture<ExitReason, Resolve<'a, 'config, H>>> {
        step!(self, handler, return Err; Ok)
    }

    /// Loop stepping the runtime until it stops.
    pub fn run<'a, H: Handler>(
        &'a mut self,
        handler: &mut H,
    ) -> Capture<ExitReason, Resolve<'a, 'config, H>> {
        loop {
            step!(self, handler, return;)
        }
    }
}

/// Runtime configuration.
#[derive(Clone, Debug)]
pub struct Config {
    /// Gas paid for extcode.
    pub gas_ext_code: usize,
    /// Gas paid for extcodehash.
    pub gas_ext_code_hash: usize,
    /// Gas paid for sstore set.
    pub gas_sstore_set: usize,
    /// Gas paid for sstore reset.
    pub gas_sstore_reset: usize,
    /// Gas paid for sstore refund.
    pub refund_sstore_clears: isize,
    /// Gas paid for BALANCE opcode.
    pub gas_balance: usize,
    /// Gas paid for SLOAD opcode.
    pub gas_sload: usize,
    /// Gas paid for SUICIDE opcode.
    pub gas_suicide: usize,
    /// Gas paid for SUICIDE opcode when it hits a new account.
    pub gas_suicide_new_account: usize,
    /// Gas paid for CALL opcode.
    pub gas_call: usize,
    /// Gas paid for EXP opcode for every byte.
    pub gas_expbyte: usize,
    /// Gas paid for a contract creation transaction.
    pub gas_transaction_create: usize,
    /// Gas paid for a message call transaction.
    pub gas_transaction_call: usize,
    /// Gas paid for zero data in a transaction.
    pub gas_transaction_zero_data: usize,
    /// Gas paid for non-zero data in a transaction.
    pub gas_transaction_non_zero_data: usize,
    /// EIP-1283.
    pub sstore_gas_metering: bool,
    /// EIP-1706.
    pub sstore_revert_under_stipend: bool,
    /// Whether to throw out of gas error when
    /// CALL/CALLCODE/DELEGATECALL requires more than maximum amount
    /// of gas.
    pub err_on_call_with_more_gas: bool,
    /// Take l64 for callcreate after gas.
    pub call_l64_after_gas: bool,
    /// Whether empty account is considered exists.
    pub empty_considered_exists: bool,
    /// Whether create transactions and create opcode increases nonce by one.
    pub create_increase_nonce: bool,
    /// Stack limit.
    pub stack_limit: usize,
    /// Memory limit.
    pub memory_limit: usize,
    /// Call limit.
    pub call_stack_limit: usize,
    /// Create contract limit.
    pub create_contract_limit: Option<usize>,
    /// Call stipend.
    pub call_stipend: usize,
    /// Has delegate call.
    pub has_delegate_call: bool,
    /// Has create2.
    pub has_create2: bool,
    /// Has revert.
    pub has_revert: bool,
    /// Has return data.
    pub has_return_data: bool,
    /// Has bitwise shifting.
    pub has_bitwise_shifting: bool,
    /// Has chain ID.
    pub has_chain_id: bool,
    /// Has self balance.
    pub has_self_balance: bool,
    /// Has ext code hash.
    pub has_ext_code_hash: bool,
}

impl Config {
    /// Frontier hard fork configuration.
    pub const fn frontier() -> Config {
        Config {
            gas_ext_code: 20,
            gas_ext_code_hash: 20,
            gas_balance: 20,
            gas_sload: 50,
            gas_sstore_set: 20000,
            gas_sstore_reset: 5000,
            refund_sstore_clears: 15000,
            gas_suicide: 0,
            gas_suicide_new_account: 0,
            gas_call: 40,
            gas_expbyte: 10,
            gas_transaction_create: 21000,
            gas_transaction_call: 21000,
            gas_transaction_zero_data: 4,
            gas_transaction_non_zero_data: 68,
            sstore_gas_metering: false,
            sstore_revert_under_stipend: false,
            err_on_call_with_more_gas: true,
            empty_considered_exists: true,
            create_increase_nonce: false,
            call_l64_after_gas: false,
            stack_limit: 1024,
            memory_limit: usize::max_value(),
            call_stack_limit: 1024,
            create_contract_limit: None,
            call_stipend: 2300,
            has_delegate_call: false,
            has_create2: false,
            has_revert: false,
            has_return_data: false,
            has_bitwise_shifting: false,
            has_chain_id: false,
            has_self_balance: false,
            has_ext_code_hash: false,
        }
    }

    /// Istanbul hard fork configuration.
    pub const fn istanbul() -> Config {
        Config {
            gas_ext_code: 700,
            gas_ext_code_hash: 700,
            gas_balance: 700,
            gas_sload: 800,
            gas_sstore_set: 20000,
            gas_sstore_reset: 5000,
            refund_sstore_clears: 15000,
            gas_suicide: 5000,
            gas_suicide_new_account: 25000,
            gas_call: 700,
            gas_expbyte: 50,
            gas_transaction_create: 53000,
            gas_transaction_call: 21000,
            gas_transaction_zero_data: 4,
            gas_transaction_non_zero_data: 16,
            sstore_gas_metering: true,
            sstore_revert_under_stipend: true,
            err_on_call_with_more_gas: false,
            empty_considered_exists: false,
            create_increase_nonce: true,
            call_l64_after_gas: true,
            stack_limit: 1024,
            memory_limit: usize::max_value(),
            call_stack_limit: 1024,
            create_contract_limit: Some(0x6000),
            call_stipend: 2300,
            has_delegate_call: true,
            has_create2: true,
            has_revert: true,
            has_return_data: true,
            has_bitwise_shifting: true,
            has_chain_id: true,
            has_self_balance: true,
            has_ext_code_hash: true,
        }
    }
}
