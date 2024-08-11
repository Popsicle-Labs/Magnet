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

//! Benchmarking setup for pallet-order
#![cfg(feature = "runtime-benchmarks")]
pub use super::*;

#[allow(unused)]
use crate::Pallet as Order;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_consensus_aura::sr25519::AuthorityId;
use sp_runtime::Perbill;

benchmarks! {
	where_clause { where T::AccountId : From<[u8; 32]>,}

	set_slot_width {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, 3)
	verify {
		assert_eq!(SlotWidth::<T>::get(), 3);

	}

	set_price_limit {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
		let price_limit = BalanceOf::<T>::from(1000000000 as u32);
	}: _(RawOrigin::Root, price_limit)
	verify {
		assert_eq!(PriceLimit::<T>::get(), price_limit);
	}

	set_gas_threshold {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
		let threshold = Perbill::from_percent(10);
	}: _(RawOrigin::Root, threshold)
	verify {
		assert_eq!(GasThreshold::<T>::get(), threshold);
	}

	create_order {
		let s in 0..100;
		let r = [0xd4,0x35,0x93,0xc7,0x15,0xfd,0xd3,0x1c,0x61,0x14,0x1a,0xbd,0x04,0xa9,0x9f,0xd6,0x82,0x2c,0x85,0x58,0x85,0x4c,0xcd,0xe3,0x9a,0x56,0x84,0xe7,0xa5,0x6d,0xa2,0x7d];
		let author_pub = AuthorityId::from(sp_core::sr25519::Public::from_raw(r));
		let bulk_inherent_data = mp_coretime_on_demand::OrderInherentData {
			relay_chian_number: 40,
			author_pub: Some(author_pub),
			price: 10000000,
		};

	}: _(RawOrigin::None, bulk_inherent_data)
	verify {
		assert_eq!(SequenceNumber::<T>::get(), 1);
	}
}

impl_benchmark_test_suite!(Order, crate::mock::ExtBuilder::default().build(), crate::mock::Test,);
