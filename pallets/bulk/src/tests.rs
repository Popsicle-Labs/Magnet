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

use crate::mock::*;
use crate::proof_data::{STORAGE_PROOF, STORAGE_ROOT};
use frame_support::{
	inherent::{InherentData, ProvideInherent},
	traits::UnfilteredDispatchable,
};
use frame_system::RawOrigin;
use pallet_broker::{CoreMask, RegionId};
use sp_trie::StorageProof;

#[test]
fn bulk_inherent_test() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let root = hex::decode(STORAGE_ROOT[0]).unwrap();
		let coretime_chain_state_proof =
			StorageProof::new(STORAGE_PROOF.iter().map(|s| hex::decode(s).unwrap()));
		let storage_root: cumulus_primitives_core::relay_chain::Hash =
			<[u8; 32]>::try_from(root).unwrap().into();
		let core_mask = CoreMask::from(0xFFFFFFFFFFFFFFFFFFFF);
		let region_id = RegionId { begin: 13, core: 1, mask: core_mask };
		let mut inherent_data = InherentData::default();
		let bulk_inherent_data = mp_coretime_bulk::BulkInherentData {
			storage_proof: Some(coretime_chain_state_proof),
			storage_root,
			region_id,
			duration: 100,
			start_relaychain_height: 130,
			end_relaychain_height: 230,
			record_index: 0,
		};
		inherent_data
			.put_data(mp_coretime_bulk::INHERENT_IDENTIFIER, &bulk_inherent_data)
			.expect("failed to put VFP inherent");
		BulkPallet::create_inherent(&inherent_data)
			.expect("got an inherent")
			.dispatch_bypass_filter(RawOrigin::None.into())
			.expect("dispatch succeeded");
		BulkPallet::on_finalize(1);
		assert_eq!(BulkPallet::record_index(), 1);
		let record = BulkPallet::bulk_records(0).unwrap();
		assert_eq!(record.start_relaychain_height, 130);
		assert_eq!(record.end_relaychain_height, 230);
	});
}
