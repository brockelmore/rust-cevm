use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use primitive_types::{H160, H256, U256};
use sha3::{Digest, Keccak256};
use super::{Basic, Backend, ApplyBackend, Apply, Log, MemoryVicinity, MemoryAccount, TxReceipt};
// #[cfg(feature = "web")]
// use crate::provider::webprovider::Provider;

#[cfg(feature = "local")]
use crate::provider::localprovider::Provider;




/// Memory backend with ability to fork another chain from an HTTP provider, storing all state values in a `BTreeMap` in memory.
#[derive(Clone, Debug)]
pub struct ForkMemoryBackendOwned {
	/// backend vicinity
	pub vicinity: MemoryVicinity,
	state: BTreeMap<H160, MemoryAccount>,
	archive_state: BTreeMap<U256, BTreeMap<H160, MemoryAccount>>,
	logs: BTreeMap<U256, Vec<Log>>,
	provider: Provider,
	local_block_num: U256,
	tx_history: BTreeMap<H256, TxReceipt>
}

impl ForkMemoryBackendOwned {
	/// Create a new memory backend.
	pub fn new(vicinity: MemoryVicinity, state: BTreeMap<H160, MemoryAccount>, provider: String) -> Self {
		Self {
			vicinity: vicinity.clone(),
			state,
			archive_state: BTreeMap::new(),
			logs: BTreeMap::new(),
			provider: Provider::new(provider),
			local_block_num: vicinity.block_number.clone(),
			tx_history: BTreeMap::new()
		}
	}

	/// Get the underlying `BTreeMap` storing the state.
	pub fn state(&self) -> &BTreeMap<H160, MemoryAccount> {
		&self.state
	}
}

impl Backend for ForkMemoryBackendOwned {
	fn gas_price(&self) -> U256 { self.vicinity.gas_price }
	fn origin(&self) -> H160 { self.vicinity.origin }
	fn block_hash(&self, number: U256) -> H256 {
		if number >= self.vicinity.block_number ||
			self.vicinity.block_number - number - U256::one() >= U256::from(self.vicinity.block_hashes.len())
		{
			H256::default()
		} else {
			let index = (self.vicinity.block_number - number - U256::one()).as_usize();
			self.vicinity.block_hashes[index]
		}
	}
	fn block_number(&self) -> U256 { self.local_block_num }
	fn block_coinbase(&self) -> H160 { self.vicinity.block_coinbase }
	fn block_timestamp(&self) -> U256 { self.vicinity.block_timestamp }
	fn block_difficulty(&self) -> U256 { self.vicinity.block_difficulty }
	fn block_gas_limit(&self) -> U256 { self.vicinity.block_gas_limit }

	fn chain_id(&self) -> U256 { self.vicinity.chain_id }

	fn exists(&self, address: H160) -> bool {
		self.state.contains_key(&address) || (
			(self.provider
				.get_balance(address, Some(self.vicinity.block_number)) != U256::default())
			|| (self.provider
				.get_transaction_count(address, Some(self.vicinity.block_number)) != U256::default())
			|| (self.provider
				.get_code(address, Some(self.vicinity.block_number)).as_ref().to_vec() != Vec::<u8>::new())
		)
	}

	fn basic(&self, address: H160) -> Basic {
		self.state.get(&address).map(|a| {
			Basic { balance: a.balance, nonce: a.nonce }
		}).unwrap_or_else(|| {
			Basic {
				balance: self.provider.get_balance(address, Some(self.vicinity.block_number)),
				nonce: self.provider.get_transaction_count(address, Some(self.vicinity.block_number))
			}
		})
	}

	fn code_hash(&self, address: H160) -> H256 {
		self.state.get(&address).map(|v| {
			H256::from_slice(Keccak256::digest(&v.code).as_slice())
		}).unwrap_or_else( || {
			let code = self.provider.get_code(address, Some(self.vicinity.block_number));
			H256::from_slice(Keccak256::digest(&code.as_ref().to_vec()).as_slice())
		})
	}

	fn code_size(&self, address: H160) -> usize {
		self.state.get(&address).map(|v| v.code.len()).unwrap_or_else( || {
			let code = self.provider.get_code(address, Some(self.vicinity.block_number));
			code.as_ref().to_vec().len()
		})
	}

	fn code(&self, address: H160) -> Vec<u8> {
		self.state.get(&address).map(|v| v.code.clone()).unwrap_or_else( || {
			self.provider.get_code(address, Some(self.vicinity.block_number)).as_ref().to_vec()
		})
	}

	fn storage(&self, address: H160, index: H256) -> H256 {
		if let Some(acct) = self.state.get(&address) {
			if let Some(store_data) = acct.storage.get(&index) {
				store_data.clone()
			} else {
				self.provider.get_storage_at(address, index, Some(self.vicinity.block_number))
			}
		} else {
			self.provider.get_storage_at(address, index, Some(self.vicinity.block_number))
		}
	}

	fn tx_receipt(&self, hash: H256) -> TxReceipt {
        if let Some(txrec) = self.tx_history.get(&hash) {
			txrec.clone()
		} else {
			self.provider.get_transaction_receipt(hash)
		}
    }
}

impl ApplyBackend for ForkMemoryBackendOwned {
	fn apply<A, I, L>(
		&mut self,
		block: U256,
		values: A,
		logs: L,
		recs: Vec<TxReceipt>,
		delete_empty: bool,
	) where
		A: IntoIterator<Item=Apply<I>>,
		I: IntoIterator<Item=(H256, H256)>,
		L: IntoIterator<Item=Log>,
	{
		let mut tip = false;
		if block == self.local_block_num {
			tip = true;
		}
		for apply in values {
			match apply {
				Apply::Modify {
					address, basic, code, storage, reset_storage,
				} => {
					let is_empty = {

						if tip {
							let account = self.state.entry(address).or_insert(Default::default());
							account.balance = basic.balance;
							account.nonce = basic.nonce;
							if let Some(code) = code {
								account.code = code;
							}

							if reset_storage {
								account.storage = BTreeMap::new();
							}

							let zeros = account.storage.iter()
								.filter(|(_, v)| v == &&H256::default())
								.map(|(k, _)| k.clone())
								.collect::<Vec<H256>>();

							for zero in zeros {
								account.storage.remove(&zero);
							}

							for (index, value) in storage {
								if value == H256::default() {
									account.storage.remove(&index);
								} else {
									account.storage.insert(index, value);
								}
							}

							account.balance == U256::zero() &&
								account.nonce == U256::zero() &&
								account.code.len() == 0
						} else {
							// changes arent for this blocking
							let archive = self.archive_state.entry(block).or_insert(Default::default());
							let account = archive.entry(address).or_insert(Default::default());
							account.balance = basic.balance;
							account.nonce = basic.nonce;
							if let Some(code) = code {
								account.code = code;
							}

							if reset_storage {
								account.storage = BTreeMap::new();
							}

							let zeros = account.storage.iter()
								.filter(|(_, v)| v == &&H256::default())
								.map(|(k, _)| k.clone())
								.collect::<Vec<H256>>();

							for zero in zeros {
								account.storage.remove(&zero);
							}

							for (index, value) in storage {
								if value == H256::default() {
									account.storage.remove(&index);
								} else {
									account.storage.insert(index, value);
								}
							}

							account.balance == U256::zero() &&
								account.nonce == U256::zero() &&
								account.code.len() == 0
						}
					};

					if is_empty && delete_empty {
						self.state.remove(&address);
					}
				},
				Apply::Delete {
					address,
				} => {
					self.state.remove(&address);
				},
			}
		}

		let ls = self.logs.entry(block).or_insert(Vec::new());
		let mut f = ls.clone();
		for log in logs {
			f.push(log);
		}
		*ls = f;

		for rec in recs {
			self.tx_history.insert(rec.hash, rec);
		}
	}
}
