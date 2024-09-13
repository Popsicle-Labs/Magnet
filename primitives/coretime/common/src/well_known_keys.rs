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

use {cumulus_primitives_core::ParaId, sp_core::Encode, sp_io::hashing::twox_64, sp_std::vec::Vec};

pub const PARAS_PARA_LIFECYCLES: &[u8] =
	&hex_literal::hex!["cd710b30bd2eab0352ddcc26417aa194281e0bfde17b36573208a06cb5cfba6b"];

// Paras pallet storage ParaLifecycles
pub fn paras_para_lifecycles(para_id: ParaId) -> Vec<u8> {
	para_id.using_encoded(|para_id: &[u8]| {
		PARAS_PARA_LIFECYCLES
			.iter()
			.chain(twox_64(para_id).iter())
			.chain(para_id.iter())
			.cloned()
			.collect()
	})
}

// System pallet BlockHash block number 0
pub const SYSTEM_BLOCKHASH_GENESIS: &[u8] = &hex_literal::hex![
	"26aa394eea5630e07c48ae0c9558cef7a44704b568d21667356a5a050c118746b4def25cfda6ef3a00000000"
];

// System pallet BlockHash
pub const SYSTEM_BLOCKHASH: &[u8] =
	&hex_literal::hex!["26aa394eea5630e07c48ae0c9558cef7a44704b568d21667356a5a050c118746"];

// System pallet BlockHash
pub fn chain_block_hash(block_number: u32) -> Vec<u8> {
	block_number.using_encoded(|block_number: &[u8]| {
		SYSTEM_BLOCKHASH
			.iter()
			.chain(twox_64(block_number).iter())
			.chain(block_number.iter())
			.cloned()
			.collect()
	})
}

// configuration pallet version
pub const CONFIGURATION_VERSION: &[u8] =
	&hex_literal::hex!["06de3d8a54d27e44a9d5ce189618f22d4e7b9012096b41c4eb3aaf947f6ea429"];

//Configuration ActiveConfig
pub const ACTIVE_CONFIG: &[u8] =
	&hex_literal::hex!["06de3d8a54d27e44a9d5ce189618f22db4b49d95320d9021994c850f25b8e385"];

// on demand pallet version
pub const ON_DEMAND_VERSION: &[u8] =
	&hex_literal::hex!["8f32430b49607f8d60bfd3a003ddf4b54e7b9012096b41c4eb3aaf947f6ea429"];

//OnDemandAssignmentProvider SpotTraffic
pub const SPOT_TRAFFIC: &[u8] =
	&hex_literal::hex!["8f32430b49607f8d60bfd3a003ddf4b5c9308a8e0e640735727536bd9069b11e"];

//OnDemandAssignmentProvider QueueStatusType
pub const QUEUE_STATUS_TYPE: &[u8] =
	&hex_literal::hex!["8f32430b49607f8d60bfd3a003ddf4b58bf29330833ea7904c7209f4ce9d917a"];
