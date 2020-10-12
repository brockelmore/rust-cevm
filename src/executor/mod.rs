//! # EVM executors
//!
//! Executors are structs that hook gasometer and the EVM core together. It
//! also handles the call stacks in EVM.

mod stack;
// mod stack_owned;

pub use self::stack::{StackAccount, StackExecutor};
// pub use self::stack_owned::StackExecutorOwned;
