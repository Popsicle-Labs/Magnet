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

//! # Bulk Inherent Primitives
//!
//! This crate defines those primitives that should be taken into account when building
//! the bulk pallet inherent
//!
#![cfg_attr(not(feature = "std"), no_std)]
use cumulus_primitives_core::relay_chain::Hash as PHash;
use sp_runtime::sp_std::vec::Vec;
#[cfg(feature = "std")]
pub mod inherent_client;
pub mod well_known_keys;
use codec::{Decode, Encode};
use pallet_broker::{RegionId, Timeslice};
use sp_core::MaxEncodedLen;
use sp_runtime::RuntimeDebug;
use {scale_info::TypeInfo, sp_inherents::InherentIdentifier};

/// Inherent data of bulk mode.
#[derive(Encode, Decode, sp_core::RuntimeDebug, Clone, PartialEq, TypeInfo)]
pub struct BulkInherentData {
	/// Proof of coretime parachain storage.
	pub storage_proof: Option<sp_trie::StorageProof>,
	/// Root of coretime parachain storage.
	pub storage_root: PHash,
	/// The identity of the Region.
	pub region_id: RegionId,
	/// Coretime duration.
	pub duration: u32,
	/// Relaychain block number of start schedule coretime core.
	pub start_relaychain_height: u32,
	/// Relaychain block number of end schedule coretime core.
	pub end_relaychain_height: u32,
}

/// Status of bulk purchased then assigned.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BulkStatus {
	/// User call broker purchase.
	Purchased,
	/// User call broker assign.
	Assigned,
	/// broker do_tick().
	CoreAssigned,
}

#[derive(Clone, Debug)]
pub struct BulkMemRecordItem {
	/// Proof of coretime parachain storage.
	pub storage_proof: sp_trie::StorageProof,
	/// Root of coretime parachain storage.
	pub storage_root: PHash,
	/// The identity of the Region.
	pub region_id: RegionId,
	/// Relaychain block number of start schedule coretime core.
	pub start_relaychain_height: u32,
	/// Relaychain block number of end schedule coretime core.
	pub end_relaychain_height: u32,
	/// Coretime duration.
	pub duration: u32,
	/// Status of bulk record.
	pub status: BulkStatus,
}

#[derive(Clone, Debug)]
pub struct BulkMemRecord {
	/// Block height of coretime parachain.
	pub coretime_para_height: u32,
	/// Record item.
	pub items: Vec<BulkMemRecordItem>,
}

// Identifier of the bulk inherent
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"bulkihrt";

sp_api::decl_runtime_apis! {
	#[api_version(2)]
	pub trait BulkRuntimeApi
	{
		// Rpc url of coretime parachain.
		fn rpc_url() -> Vec<u8>;
		// Block number of relaychain.
		fn relaychain_block_number()->u32;
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RegionRecord<AccountId, Balance> {
	/// The end of the Region.
	pub end: Timeslice,
	/// The owner of the Region.
	pub owner: Option<AccountId>,
	/// The amount paid to Polkadot for this Region, or `None` if renewal is not allowed.
	pub paid: Option<Balance>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RegionRecordV0<AccountId, Balance> {
	/// The end of the Region.
	pub end: Timeslice,
	/// The owner of the Region.
	pub owner: AccountId,
	/// The amount paid to Polkadot for this Region, or `None` if renewal is not allowed.
	pub paid: Option<Balance>,
}
