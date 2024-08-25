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

use pallet_broker::RegionId;
use {crate::BulkInherentData, cumulus_relay_chain_interface::PHash};

impl BulkInherentData {
	/// Create the [`BulkInherentData`] at the given `relay_parent`.
	///
	/// Returns `None` if the creation failed.
	pub async fn create_at(
		storage_proof: Option<&sp_trie::StorageProof>,
		storage_root: PHash,
		region_id: RegionId,
		duration: u32,
		start: u32,
		end: u32,
		record_index: u32,
	) -> Option<BulkInherentData> {
		let storage_proof =
			if let Some(proof) = storage_proof { Some(proof.clone()) } else { None };
		Some(BulkInherentData {
			storage_proof,
			storage_root,
			region_id,
			duration,
			start_relaychain_height: start,
			end_relaychain_height: end,
			record_index,
		})
	}
}

// Implementation of InherentDataProvider
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for BulkInherentData {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(crate::INHERENT_IDENTIFIER, &self)
	}

	async fn try_handle_error(
		&self,
		_: &sp_inherents::InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		None
	}
}
