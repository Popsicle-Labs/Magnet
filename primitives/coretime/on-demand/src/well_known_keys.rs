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

//! Keys of well known.
#![cfg_attr(not(feature = "std"), no_std)]

use crate::{Decode, ParaId};
use cumulus_primitives_core::relay_chain::CoreIndex;
use sp_runtime::AccountId32;
use {
	sp_core::Encode,
	sp_io::hashing::{blake2_128, twox_256},
	sp_std::vec::Vec,
};
pub const SYSTEM_ACCOUNT: &[u8] =
	&hex_literal::hex!["26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9"];

pub const SYSTEM_EVENTS: &[u8] =
	&hex_literal::hex!["26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"];

//OnDemandAssignmentProvider OnDemandQueue
pub const ON_DEMAND_QUEUE: &[u8] =
	&hex_literal::hex!["8f32430b49607f8d60bfd3a003ddf4b53f35b69d817556cf6b886e5b4f01fbdc"];

pub const CORE_DESCRIPTORS: &[u8] =
	&hex_literal::hex!["638595eebaa445ce03a13547bece90e704e6ac775a3245623103ffec2cb2c92f"];

/// assigner coretime storage CoreDescriptors
pub fn paras_core_descriptors(core_index: CoreIndex) -> Vec<u8> {
	core_index.using_encoded(|core_index: &[u8]| {
		CORE_DESCRIPTORS.iter().chain(twox_256(core_index).iter()).cloned().collect()
	})
}

// ParaScheduler AvailabilityCores
pub const AVAILABILITY_CORES: &[u8] =
	&hex_literal::hex!["94eadf0156a8ad5156507773d0471e4ab8ebad86f546c7e0b135a4212aace339"];

pub const BALANCE_ACCOUNT: &[u8] =
	&hex_literal::hex!["c2261276cc9d1f8598ea4b6a74b15c2fb99d880ec681799c0cf30e8886371da9"];

pub fn acount_balance(account: AccountId32) -> Vec<u8> {
	account.using_encoded(|account_bytes: &[u8]| {
		SYSTEM_ACCOUNT
			.iter()
			.chain(blake2_128(account_bytes).iter())
			.chain(account_bytes.iter())
			.cloned()
			.collect()
	})
}

#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct EnqueuedOrder {
	/// Parachain ID
	pub para_id: ParaId,
}
