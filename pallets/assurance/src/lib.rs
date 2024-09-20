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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_support::traits::Get;
use frame_support::weights::Weight;
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use cumulus_pallet_parachain_system::{RelaychainDataProvider, RelaychainStateProvider};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: cumulus_pallet_parachain_system::Config + frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Threshod for force bid coretime
		type DefaultBidThreshold: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewBidThreshold(u32),
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub bid_threshold: u32,
		#[serde(skip)]
		pub _marker: PhantomData<T>,
	}

	impl<T: Config> GenesisConfig<T> {
		pub fn new(bid_threshold: u32) -> Self {
			Self { bid_threshold, _marker: PhantomData }
		}
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { bid_threshold: T::DefaultBidThreshold::get(), _marker: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			<BidThreshold<T>>::put(self.bid_threshold);
		}
	}

	#[pallet::type_value]
	pub fn DefaultBidThreshold<T: Config>() -> u32 {
		T::DefaultBidThreshold::get()
	}

	#[pallet::storage]
	pub type BidThreshold<T> = StorageValue<_, u32, ValueQuery, DefaultBidThreshold<T>>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn set_bid_threshold(origin: OriginFor<T>, blocknumber: u32) -> DispatchResult {
			ensure_root(origin)?;
			let _ = Self::set_bid_threshold_inner(blocknumber);
			Self::deposit_event(Event::NewBidThreshold(blocknumber));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn set_bid_threshold_inner(value: u32) -> Weight {
			<BidThreshold<T>>::put(value);
			T::DbWeight::get().writes(1)
		}

		pub fn on_relaychain(blocknumber: u32) -> bool {
			let last_relay_block_number =
				RelaychainDataProvider::<T>::current_relay_chain_state().number;
			if blocknumber > BidThreshold::<T>::get() + u32::from(last_relay_block_number) {
				return true;
			}

			false
		}
	}
}
