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

use cumulus_pallet_parachain_system::{
	consensus_hook::{ConsensusHook, UnincludedSegmentCapacity},
	relay_state_snapshot::RelayChainStateProof,
};
use cumulus_primitives_core::ParaId;
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	parameter_types,
	traits::{ConstU32, ConstU64, Get},
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use sp_std::{cell::RefCell, num::NonZeroU32};

use crate as pallet_assurance;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Assurance: pallet_assurance::{Pallet, Call, Config<T>, Storage, Event<T>},
	}
);

parameter_types! {
	pub(crate) static ExtrinsicBaseWeight: Weight = Weight::zero();
	pub(crate) static ExistentialDeposit: u64 = 0;
}

pub struct BlockWeights;
impl Get<frame_system::limits::BlockWeights> for BlockWeights {
	fn get() -> frame_system::limits::BlockWeights {
		frame_system::limits::BlockWeights::builder()
			.base_block(Weight::zero())
			.for_class(DispatchClass::all(), |weights| {
				weights.base_extrinsic = ExtrinsicBaseWeight::get().into();
			})
			.for_class(DispatchClass::non_mandatory(), |weights| {
				weights.max_total = Weight::from_parts(1024, u64::MAX).into();
			})
			.build_or_panic()
	}
}

pub type AccountId = AccountId32;

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type Balance = u64;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

/// We allow for 0.5 of a second of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ParachainId: ParaId = ParaId::new(200);
}

impl cumulus_pallet_parachain_system::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = ParachainId;
	type OutboundXcmpMessageSource = ();
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = ();
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = cumulus_pallet_parachain_system::AnyRelayNumber;
	type ConsensusHook = TestConsensusHook;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<(), sp_core::ConstU8<0>>;
	type WeightInfo = ();
}

std::thread_local! {
	pub static CONSENSUS_HOOK: RefCell<Box<dyn Fn(&RelayChainStateProof) -> (Weight, UnincludedSegmentCapacity)>>
		= RefCell::new(Box::new(|_| (Weight::zero(), NonZeroU32::new(1).unwrap().into())));
}

pub struct TestConsensusHook;

impl ConsensusHook for TestConsensusHook {
	fn on_state_proof(s: &RelayChainStateProof) -> (Weight, UnincludedSegmentCapacity) {
		CONSENSUS_HOOK.with(|f| f.borrow_mut()(s))
	}
}

impl pallet_assurance::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type DefaultBidThreshold = ConstU32<8u32>;
}

pub struct ExtBuilder {
	existential_deposit: u64,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { existential_deposit: 1 }
	}
}

impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}
	pub fn set_associated_consts(&self) {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
	}
	pub fn build(self) -> sp_io::TestExternalities {
		self.set_associated_consts();
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		pallet_balances::GenesisConfig::<Test> { balances: vec![] }
			.assimilate_storage(&mut t)
			.unwrap();
		pallet_assurance::GenesisConfig::<Test> {
			bid_threshold: 8u32,
			_marker: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();
		cumulus_pallet_parachain_system::GenesisConfig::<Test>::default()
			.build_storage()
			.unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn last_event() -> RuntimeEvent {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

pub(crate) fn expect_event<E: Into<RuntimeEvent>>(e: E) {
	assert_eq!(last_event(), e.into());
}
