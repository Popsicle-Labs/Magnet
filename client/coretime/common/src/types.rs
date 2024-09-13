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

#[derive(
	:: subxt :: ext :: subxt_core :: ext :: codec :: CompactAs,
	:: subxt :: ext :: subxt_core :: ext :: codec :: Decode,
	:: subxt :: ext :: subxt_core :: ext :: codec :: Encode,
	:: subxt :: ext :: subxt_core :: ext :: scale_decode :: DecodeAsType,
	:: subxt :: ext :: subxt_core :: ext :: scale_encode :: EncodeAsType,
	Debug,
)]
# [codec (crate = :: subxt :: ext :: subxt_core :: ext :: codec)]
#[decode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_encode")]
pub struct QueueIndex(pub ::core::primitive::u32);

#[derive(
	:: subxt :: ext :: subxt_core :: ext :: codec :: CompactAs,
	:: subxt :: ext :: subxt_core :: ext :: codec :: Decode,
	:: subxt :: ext :: subxt_core :: ext :: codec :: Encode,
	:: subxt :: ext :: subxt_core :: ext :: scale_decode :: DecodeAsType,
	:: subxt :: ext :: subxt_core :: ext :: scale_encode :: EncodeAsType,
	Debug,
)]
# [codec (crate = :: subxt :: ext :: subxt_core :: ext :: codec)]
#[decode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_encode")]
pub struct ReverseQueueIndex(pub ::core::primitive::u32);

#[derive(
	:: subxt :: ext :: subxt_core :: ext :: codec :: CompactAs,
	:: subxt :: ext :: subxt_core :: ext :: codec :: Decode,
	:: subxt :: ext :: subxt_core :: ext :: codec :: Encode,
	:: subxt :: ext :: subxt_core :: ext :: scale_decode :: DecodeAsType,
	:: subxt :: ext :: subxt_core :: ext :: scale_encode :: EncodeAsType,
	Debug,
)]
# [codec (crate = :: subxt :: ext :: subxt_core :: ext :: codec)]
#[decode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_encode")]
pub struct FixedU128(pub ::core::primitive::u128);

#[derive(
	:: subxt :: ext :: subxt_core :: ext :: codec :: Decode,
	:: subxt :: ext :: subxt_core :: ext :: codec :: Encode,
	:: subxt :: ext :: subxt_core :: ext :: scale_decode :: DecodeAsType,
	:: subxt :: ext :: subxt_core :: ext :: scale_encode :: EncodeAsType,
	Debug,
)]
# [codec (crate = :: subxt :: ext :: subxt_core :: ext :: codec)]
#[decode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_decode")]
#[encode_as_type(crate_path = ":: subxt :: ext :: subxt_core :: ext :: scale_encode")]
pub struct QueueStatusType {
	pub traffic: FixedU128,
	pub next_index: QueueIndex,
	pub smallest_index: QueueIndex,
	pub freed_indices: ::subxt::ext::subxt_core::alloc::vec::Vec<ReverseQueueIndex>,
}
