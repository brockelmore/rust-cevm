#![allow(non_camel_case_types)]
use actix::prelude::*;
use evm::backend::memory::TxReceipt;
use evm::executor::CallTrace;
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};
use web3::types::{TransactionReceipt, TransactionRequest};

use hash::keccak;
use parity_crypto::publickey::{recover, Public, Signature};
use rlp::{self, DecoderError, Encodable, Rlp, RlpStream};
use std::ops::Deref;
use web3::types::*;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum TX {
    Hashes(Vec<H256>),
    Full(Vec<TransactionReceipt>),
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Block {
    /// Hash of the block
    pub hash: Option<H256>,
    /// Hash of the parent
    #[serde(rename = "parentHash")]
    pub parent_hash: H256,
    /// Hash of the uncles
    #[cfg(not(feature = "celo"))]
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: H256,
    /// Miner/author's address.
    #[serde(rename = "miner")]
    pub author: H160,
    /// State root hash
    #[serde(rename = "stateRoot")]
    pub state_root: H256,
    /// Transactions root hash
    #[serde(rename = "transactionsRoot")]
    pub transactions_root: H256,
    /// Transactions receipts root hash
    #[serde(rename = "receiptsRoot")]
    pub receipts_root: H256,
    /// Block number. None if pending.
    pub number: Option<U256>,
    /// Gas Used
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    /// Gas Limit
    #[cfg(not(feature = "celo"))]
    #[serde(rename = "gasLimit")]
    pub gas_limit: U256,
    /// Extra data
    #[serde(rename = "extraData")]
    pub extra_data: Vec<u8>,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: Option<Vec<u8>>,
    /// Timestamp
    pub timestamp: U256,
    /// Difficulty
    #[cfg(not(feature = "celo"))]
    pub difficulty: U256,
    /// Total difficulty
    #[serde(rename = "totalDifficulty")]
    pub total_difficulty: Option<U256>,
    /// Seal fields
    #[serde(default, rename = "sealFields")]
    pub seal_fields: Vec<Vec<u8>>,
    /// Uncles' hashes
    pub uncles: Vec<H256>,
    /// Transactions
    pub transactions: Vec<TX>,
    /// Size in bytes
    pub size: Option<U256>,
}

pub enum EthRequest {
    eth_accounts,
    eth_blockNumber,
    eth_getBalance(H160, Option<U256>),
    eth_getStorageAt(H160, U256, Option<U256>),
    eth_getTransactionCount(H160, Option<U256>),
    eth_getCode(H160, Option<U256>),
    eth_sendTransaction(TransactionRequest, Option<Vec<String>>),
    eth_tmpDeploy(TransactionRequest, Option<Vec<String>>),
    eth_sendRawTransaction(Vec<u8>),
    eth_call(TransactionRequest, Option<U256>),
    eth_getBlockByHash(H256, bool),
    eth_getBlockByNumber(U256, bool),
    eth_getTransactionByHash(H256),
    eth_getTransactionReceipt(H256),
}

#[derive(MessageResponse, Serialize, Deserialize, Debug, Clone)]
pub enum EthResponse {
    eth_accounts(Vec<H160>),
    eth_blockNumber(U256),
    eth_getBalance(U256),
    eth_getStorageAt(H256),
    eth_getTransactionCount(U256),
    eth_getCode(Vec<u8>),
    eth_sendRawTransaction(H256),
    eth_call(Vec<u8>),
    eth_getBlockByHash(Block),
    eth_getBlockByNumber(Block),
    eth_getTransactionByHash(TxReceipt),
    eth_getTransactionReceipt(TxReceipt),
    eth_sendTransaction {
        hash: H256,
        data: Option<Vec<u8>>,
        logs: Option<Vec<evm::backend::Log>>,
        recs: Option<Vec<TxReceipt>>,
        trace: Option<Vec<Box<CallTrace>>>,
    },
    eth_unimplemented,
}

impl EthResponse {
    pub fn accounts(self) -> Option<Vec<H160>> {
        match self {
            EthResponse::eth_accounts(accts) => Some(accts),
            _ => None,
        }
    }
    pub fn blockNumber(self) -> Option<U256> {
        match self {
            EthResponse::eth_blockNumber(bn) => Some(bn),
            _ => None,
        }
    }
    pub fn balance(self) -> Option<U256> {
        match self {
            EthResponse::eth_getBalance(bal) => Some(bal),
            _ => None,
        }
    }
    pub fn storage(self) -> Option<H256> {
        match self {
            EthResponse::eth_getStorageAt(val) => Some(val),
            _ => None,
        }
    }
    pub fn tx_count(self) -> Option<U256> {
        match self {
            EthResponse::eth_getTransactionCount(cnt) => Some(cnt),
            _ => None,
        }
    }
    pub fn code(self) -> Option<Vec<u8>> {
        match self {
            EthResponse::eth_getCode(code) => Some(code),
            _ => None,
        }
    }
    pub fn tx_hash(self) -> Option<H256> {
        match self {
            EthResponse::eth_sendRawTransaction(hash) => Some(hash),
            _ => None,
        }
    }
    pub fn call(self) -> Option<Vec<u8>> {
        match self {
            EthResponse::eth_call(data) => Some(data),
            _ => None,
        }
    }
    pub fn block(self) -> Option<Block> {
        match self {
            EthResponse::eth_getBlockByHash(b) => Some(b),
            EthResponse::eth_getBlockByNumber(b) => Some(b),
            _ => None,
        }
    }
    pub fn tx(self) -> Option<TxReceipt> {
        match self {
            EthResponse::eth_getTransactionByHash(tx) => Some(tx),
            _ => None,
        }
    }
    pub fn tx_receipts(self) -> Option<Vec<TxReceipt>> {
        match self {
            EthResponse::eth_sendTransaction {
                hash,
                data,
                logs,
                recs,
                trace,
            } => recs,
            _ => None,
        }
    }
    pub fn tx_logs(self) -> Option<Vec<evm::backend::Log>> {
        match self {
            EthResponse::eth_sendTransaction {
                hash,
                data,
                logs,
                recs,
                trace,
            } => logs,
            _ => None,
        }
    }
    pub fn tx_trace(self) -> Option<Vec<Box<CallTrace>>> {
        match self {
            EthResponse::eth_sendTransaction {
                hash,
                data,
                logs,
                recs,
                trace,
            } => trace,
            _ => None,
        }
    }
    pub fn tx_data(self) -> Option<Vec<u8>> {
        match self {
            EthResponse::eth_sendTransaction {
                hash,
                data,
                logs,
                recs,
                trace,
            } => data,
            _ => None,
        }
    }
    pub fn tx_receipt(
        self,
    ) -> Option<(
        H256,
        Option<Vec<u8>>,
        Option<Vec<evm::backend::Log>>,
        Option<Vec<TxReceipt>>,
        Option<Vec<Box<CallTrace>>>,
    )> {
        match self {
            EthResponse::eth_sendTransaction {
                hash,
                data,
                logs,
                recs,
                trace,
            } => Some((hash, data, logs, recs, trace)),
            _ => None,
        }
    }
}

impl Message for EthRequest {
    type Result = EthResponse;
}

/// Signed transaction information without verified signature.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnverifiedTransaction {
    /// Plain Transaction.
    pub unsigned: SelfTransaction,
    /// The V field of the signature; the LS bit described which half of the curve our point falls
    /// in. The MS bits describe which chain this transaction is for. If 27/28, its for all chains.
    pub v: u64,
    /// The R field of the signature; helps describe the point on the curve.
    pub r: ethereum_types::U256,
    /// The S field of the signature; helps describe the point on the curve.
    pub s: ethereum_types::U256,
    /// Hash of the transaction
    pub hash: ethereum_types::H256,
}

impl Deref for UnverifiedTransaction {
    type Target = SelfTransaction;

    fn deref(&self) -> &Self::Target {
        &self.unsigned
    }
}

impl rlp::Decodable for UnverifiedTransaction {
    fn decode(d: &Rlp) -> Result<Self, DecoderError> {
        if d.item_count()? != 9 {
            return Err(DecoderError::RlpIncorrectListLen);
        }
        let hash = keccak(d.as_raw());
        Ok(UnverifiedTransaction {
            unsigned: SelfTransaction {
                nonce: d.val_at(0)?,
                gas_price: d.val_at(1)?,
                gas: d.val_at(2)?,
                action: d.val_at(3)?,
                value: d.val_at(4)?,
                data: d.val_at(5)?,
            },
            v: d.val_at(6)?,
            r: d.val_at(7)?,
            s: d.val_at(8)?,
            hash,
        })
    }
}

impl rlp::Encodable for UnverifiedTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        self.rlp_append_sealed_transaction(s)
    }
}

impl UnverifiedTransaction {
    /// Used to compute hash of created transactions
    pub fn compute_hash(mut self) -> UnverifiedTransaction {
        let hash = keccak(&*self.rlp_bytes());
        self.hash = hash;
        self
    }

    /// Returns transaction receiver, if any
    pub fn receiver(&self) -> Option<Address> {
        match self.unsigned.action {
            Action::Create => None,
            Action::Call(receiver) => Some(receiver),
        }
    }

    /// Append object with a signature into RLP stream
    fn rlp_append_sealed_transaction(&self, s: &mut RlpStream) {
        s.begin_list(9);
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.action);
        s.append(&self.value);
        s.append(&self.data);
        s.append(&self.v);
        s.append(&self.r);
        s.append(&self.s);
    }

    ///	Reference to unsigned part of this transaction.
    pub fn as_unsigned(&self) -> &SelfTransaction {
        &self.unsigned
    }

    /// Get the hash of this transaction (keccak of the RLP).
    pub fn hash(&self) -> H256 {
        self.hash
    }

    /// Returns standardized `v` value (0, 1 or 4 (invalid))
    pub fn standard_v(&self) -> u8 {
        signature::check_replay_protection(self.v)
    }

    /// The chain ID, or `None` if this is a global transaction.
    pub fn chain_id(&self) -> Option<u64> {
        match self.v {
            v if v >= 35 => Some((v - 35) / 2),
            _ => None,
        }
    }

    /// Construct a signature object from the sig.
    pub fn signature(&self) -> Signature {
        let r: ethereum_types::H256 = ethereum_types::BigEndianHash::from_uint(&self.r);
        let s: ethereum_types::H256 = ethereum_types::BigEndianHash::from_uint(&self.s);
        Signature::from_rsv(&r, &s, self.standard_v())
    }

    /// Recovers the public key of the sender.
    pub fn recover_public(&self) -> Result<Public, parity_crypto::publickey::Error> {
        Ok(recover(
            &self.signature(),
            &self.unsigned.hash(self.chain_id()),
        )?)
    }
}

/// A set of information describing an externally-originating message call
/// or contract creation operation.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SelfTransaction {
    /// Nonce.
    pub nonce: U256,
    /// Gas price.
    pub gas_price: U256,
    /// Gas paid up front for transaction execution.
    pub gas: U256,
    /// Action, can be either call or contract create.
    pub action: Action,
    /// Transfered value.
    pub value: U256,
    /// Transaction data.
    pub data: Vec<u8>,
}

impl SelfTransaction {
    /// Append object with a without signature into RLP stream
    pub fn rlp_append_unsigned_transaction(&self, s: &mut RlpStream, chain_id: Option<u64>) {
        s.begin_list(if chain_id.is_none() { 6 } else { 9 });
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.action);
        s.append(&self.value);
        s.append(&self.data);
        if let Some(n) = chain_id {
            s.append(&n);
            s.append(&0u8);
            s.append(&0u8);
        }
    }

    /// The message hash of the transaction.
    pub fn hash(&self, chain_id: Option<u64>) -> H256 {
        let mut stream = RlpStream::new();
        self.rlp_append_unsigned_transaction(&mut stream, chain_id);
        keccak(stream.as_raw())
    }
}

/// Transaction action type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Create creates new contract.
    Create,
    /// Calls contract at given address.
    /// In the case of a transfer, this is the receiver's address.'
    Call(Address),
}

impl Default for Action {
    fn default() -> Action {
        Action::Create
    }
}

impl rlp::Decodable for Action {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.is_empty() {
            if rlp.is_data() {
                Ok(Action::Create)
            } else {
                Err(DecoderError::RlpExpectedToBeData)
            }
        } else {
            Ok(Action::Call(rlp.as_val()?))
        }
    }
}

impl rlp::Encodable for Action {
    fn rlp_append(&self, s: &mut RlpStream) {
        match *self {
            Action::Create => s.append_internal(&""),
            Action::Call(ref addr) => s.append_internal(addr),
        };
    }
}

/// Replay protection logic for v part of transaction's signature
pub mod signature {
    /// Adds chain id into v
    pub fn add_chain_replay_protection(v: u64, chain_id: Option<u64>) -> u64 {
        v + if let Some(n) = chain_id {
            35 + n * 2
        } else {
            27
        }
    }

    /// Returns refined v
    /// 0 if `v` would have been 27 under "Electrum" notation, 1 if 28 or 4 if invalid.
    pub fn check_replay_protection(v: u64) -> u8 {
        match v {
            v if v == 27 => 0,
            v if v == 28 => 1,
            v if v >= 35 => ((v - 1) % 2) as u8,
            _ => 4,
        }
    }
}
