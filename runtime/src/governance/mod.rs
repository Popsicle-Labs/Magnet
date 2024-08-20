// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! New governance configurations for the Rococo runtime.

//Modified by Alex Wang 2023/11

use frame_support::{
	parameter_types,
	traits::{ConstU16, Currency, EitherOf, OnUnbalanced},
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess};
use sp_core::{
	crypto::{AccountId32, ByteArray, KeyTypeId},
	ConstU32, OpaqueMetadata, H160, H256, U256,
};
pub mod origins;
pub use origins::{
	pallet_custom_origins, AuctionAdmin, Fellows, FellowshipAdmin, FellowshipExperts,
	FellowshipInitiates, FellowshipMasters, GeneralAdmin, LeaseAdmin, ReferendumCanceller,
	ReferendumKiller, Spender, StakingAdmin, Treasurer, WhitelistedCaller,
};
pub mod tracks;
pub use tracks::TracksInfo;
pub mod fellowship;
// pub use fellowship::{FellowshipCollectiveInstance, FellowshipReferendaInstance};
use crate::{
	weights, AccountId, Balance, Balances, Block, BlockNumber, Preimage, Referenda, Runtime,
	RuntimeCall, RuntimeEvent, Scheduler, CENTS, DAYS,
};
pub use pallet_balances::{Call as BalancesCall, NegativeImbalance};
pub use parachains_common::impls::{AccountIdOf, DealWithFees};
use sp_std::marker::PhantomData;
parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 7 * DAYS;
}

impl pallet_conviction_voting::Config for Runtime {
	type WeightInfo = weights::pallet_conviction_voting::WeightInfo<Self>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout =
		frame_support::traits::tokens::currency::ActiveIssuanceOf<Balances, Self::AccountId>;
	type Polls = Referenda;
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 1 * 3 * CENTS;
	pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
}

parameter_types! {
	pub const MaxBalance: Balance = Balance::max_value();
}
pub type TreasurySpender = EitherOf<EnsureRootWithSuccess<AccountId, MaxBalance>, Spender>;

impl origins::pallet_custom_origins::Config for Runtime {}

impl pallet_whitelist::Config for Runtime {
	type WeightInfo = weights::pallet_whitelist::WeightInfo<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WhitelistOrigin =
		EitherOf<EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>, Fellows>;
	type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<Self::AccountId>, WhitelistedCaller>;
	type Preimages = Preimage;
}

pub struct MagnetToStakingPot<R>(PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for MagnetToStakingPot<R>
where
	R: pallet_balances::Config,
	AccountIdOf<R>: From<polkadot_primitives::AccountId> + Into<polkadot_primitives::AccountId>,
	<R as frame_system::Config>::RuntimeEvent: From<pallet_balances::Event<R>>,
	R::AccountId: From<AccountId32>,
{
	fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
		let staking_pot = mp_system::BASE_ACCOUNT;
		<pallet_balances::Pallet<R>>::resolve_creating(&staking_pot.into(), amount);
	}
}

impl pallet_referenda::Config for Runtime {
	type WeightInfo = weights::pallet_referenda_referenda::WeightInfo<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = frame_system::EnsureSigned<AccountId>;
	type CancelOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumCanceller>;
	type KillOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumKiller>;
	type Slash = MagnetToStakingPot<Runtime>;
	type Votes = pallet_conviction_voting::VotesOf<Runtime>;
	type Tally = pallet_conviction_voting::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
}
