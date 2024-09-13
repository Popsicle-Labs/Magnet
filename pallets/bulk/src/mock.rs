// Copyright (C) Magnet.
// This file is part of Magnet.

// Magnet is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Magnet is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Magnet.  If not, see <http://www.gnu.org/licenses/>.

use crate::{self as bulk_pallet};
use cumulus_pallet_parachain_system::{RelayChainState, RelaychainStateProvider};
pub use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{Everything, Hooks},
	BoundedVec,
};
use frame_system as system;
use frame_system::{pallet_prelude::BlockNumberFor, EnsureRoot};
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};
use std::str::FromStr;

type Block = frame_system::mocking::MockBlock<Test>;
type Signature = MultiSignature;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
type Balance = u128;
// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Event<T>},
		BulkPallet: bulk_pallet::{Pallet, Call, Storage, Event<T>},
		MockPallet: mock_pallet,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}
parameter_types! {
	pub const ExistentialDeposit: u64 = 5;
}
impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u128;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
}

pub(crate) const MOCK_RELAY_ROOT_KEY: &[u8] = b"MOCK_RELAY_ROOT_KEY";

pub struct MockRelayStateProvider;

impl RelaychainStateProvider for MockRelayStateProvider {
	fn current_relay_chain_state() -> RelayChainState {
		let root = if let Some(root) = frame_support::storage::unhashed::get(MOCK_RELAY_ROOT_KEY) {
			root
		} else {
			Default::default()
		};

		RelayChainState {
			state_root: root,
			number: 0, // block number is not relevant here
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn set_current_relay_chain_state(state: RelayChainState) {
		frame_support::storage::unhashed::put(b"MOCK_RELAY_ROOT_KEY", &state.state_root);
	}
}

parameter_types! {
	pub const MaxUrlLength: u32 = 300;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type RelayChainStateProvider = MockRelayStateProvider;
	type MaxUrlLength = MaxUrlLength;
	type WeightInfo = ();
}
pub struct BulkGasCostHandler();

pub trait BulkGasCost<T: frame_system::Config> {
	/// In Bulk mode, the average gas consumed by a block.
	///
	/// Parameters:
	/// - `block_number`: The block number of para chain.
	fn gas_cost(
		block_number: BlockNumberFor<T>,
	) -> Result<Option<(T::AccountId, Balance)>, sp_runtime::DispatchError>;
}

impl<T> BulkGasCost<T> for BulkGasCostHandler
where
	T: crate::Config,
{
	fn gas_cost(
		_block_number: BlockNumberFor<T>,
	) -> Result<Option<(T::AccountId, Balance)>, sp_runtime::DispatchError> {
		Ok(None)
	}
}

#[frame_support::pallet]
pub mod mock_pallet {
	use super::*;
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type BulkGasCost: BulkGasCost<Self>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	impl<T: Config> Pallet<T> {
		pub fn get_gas_cost(block_number: BlockNumberFor<T>) -> Option<(T::AccountId, Balance)> {
			T::BulkGasCost::gas_cost(block_number).unwrap()
		}
	}
}

impl mock_pallet::Config for Test {
	type BulkGasCost = BulkGasCostHandler;
}
pub struct ExtBuilder {
	balances: Vec<(AccountId32, u128)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: Default::default() }
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		// Build genesis storage according to the mock runtime.
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		pallet_balances::GenesisConfig::<Test> { balances: self.balances }
			.assimilate_storage(&mut t)
			.unwrap();
		crate::GenesisConfig::<Test> {
			rpc_url: BoundedVec::try_from("ws://127.0.0.1:8855".as_bytes().to_vec()).unwrap(),
			genesis_hash: H256::from_str(
				"0x4ea18c8f295ba903acbbed39c70ea0569cf1705fa954a537ffa3b8b7125eaf58",
			)
			.expect("internal U256 is valid; qed"),
			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
