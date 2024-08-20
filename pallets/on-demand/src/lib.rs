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

//! # Order Pallet
//!
//! This pallet implements the recording and query functions of purchasing ondemand core.
//!
//! By obtaining the inherent nature of the block, parsing it out of the validation_data of the relaychain,
//! and querying whether there is an OnDemandOrderPlaced event, obtaining the order account and price from the event,
//! and then writing this record to the blockchain.
//!
//! Provides many query methods for node or other pallets to use, such as querying the gas consumed by placing an order in a certain block,
//! whether the order has been executed, whether the order threshold has been reached, etc.

#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, MaxEncodedLen};
use frame_support::{
	dispatch::DispatchResultWithPostInfo, dispatch::PostDispatchInfo, pallet_prelude::*,
	traits::Currency,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use primitives::Balance;
use sp_runtime::{
	sp_std::{prelude::*, vec},
	traits::Saturating,
};
pub mod weights;
use cumulus_pallet_parachain_system::RelaychainStateProvider;
use sp_runtime::Perbill;
use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// ondemand order information.
#[derive(Encode, Decode, Default, Clone, Copy, TypeInfo, MaxEncodedLen, Debug)]
pub struct Order<AuthorityId> {
	/// The number used to record the order, incremented each time.
	pub sequence_number: u64,
	/// The height of the relay chain block that created the record.
	pub relay_chian_height: u32,
	/// The height of the relay chain block that place this order.
	pub place_order_height: u32,
	/// Account for placing order.
	pub orderer: AuthorityId,
	/// Order price.
	pub price: Balance,
}
#[frame_support::pallet]
pub mod pallet {

	use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_aura::Config
	where
		<Self as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: Currency<Self::AccountId>;

		type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type WeightInfo: WeightInfo;

		type RelayChainStateProvider: cumulus_pallet_parachain_system::RelaychainStateProvider;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	/// Sequence number,number of each order.
	#[pallet::storage]
	#[pallet::getter(fn sequence_number)]
	pub type SequenceNumber<T> = StorageValue<_, u64, ValueQuery>;

	/// The order interval is 2^slotwidth.
	#[pallet::storage]
	#[pallet::getter(fn slot_width)]
	pub(super) type SlotWidth<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The maximum price the user is willing to pay when placing an order.
	#[pallet::storage]
	#[pallet::getter(fn price_limit)]
	pub(super) type PriceLimit<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Gas threshold that triggers order placement.
	#[pallet::storage]
	#[pallet::getter(fn gas_threshold)]
	pub(super) type GasThreshold<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Order Information Map.
	#[pallet::storage]
	#[pallet::getter(fn order_map)]
	pub type OrderMap<T: Config> =
		StorageMap<_, Twox64Concat, u64, Order<T::AccountId>, OptionQuery>;

	/// Convert block height to sequence number.
	#[pallet::storage]
	#[pallet::getter(fn block_2_sequence)]
	pub type Block2Sequence<T: Config> =
		StorageMap<_, Twox64Concat, BlockNumberFor<T>, u64, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config>
	where
		T::AccountId: From<[u8; 32]>,
	{
		pub slot_width: u32,
		pub price_limit: BalanceOf<T>,
		pub gas_threshold: u128,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		fn build(&self) {
			SlotWidth::<T>::put(&self.slot_width);
			PriceLimit::<T>::put(&self.price_limit);
			GasThreshold::<T>::put(&self.gas_threshold);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		T::AccountId: From<[u8; 32]>,
	{
		/// Create order event.
		OrderCreate { sequence_number: u64, orderer: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error reading data.
		FailedReading,
		/// Order already exists.
		OrderExist,
		/// Failed to create order.
		CreateOrderFail,
		/// Invalid Validation data.
		InvalidValidation,
		/// Incorrect sequence number
		WrongSequenceNumber,
		/// Create root proof failed.
		FailedCreateProof,
		/// Slot author incorrect
		SlotAuthorError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> where T::AccountId: From<[u8; 32]> {}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		type Call = Call<T>;
		type Error = MakeFatalError<()>;

		const INHERENT_IDENTIFIER: InherentIdentifier = mp_coretime_on_demand::INHERENT_IDENTIFIER;
		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let data: mp_coretime_on_demand::OrderInherentData = data
				.get_data(&mp_coretime_on_demand::INHERENT_IDENTIFIER)
				.ok()
				.flatten()
				.expect("there is not data to be posted; qed");
			if data.author_pub.is_none() {
				None
			} else {
				Some(Call::create_order { data })
			}
		}
		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::create_order { .. })
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		/// Create an order, which is called by the pallet.
		/// Users cannot actively call this function.
		/// Obtain order information by parsing inherited data.
		///
		/// Parameters:
		/// - `data`: The inherent data.
		#[pallet::call_index(0)]
		#[pallet::weight((<T as pallet::Config>::WeightInfo::create_order(), DispatchClass::Mandatory))]
		pub fn create_order(
			origin: OriginFor<T>,
			data: mp_coretime_on_demand::OrderInherentData,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			let mp_coretime_on_demand::OrderInherentData {
				relay_chian_number: place_order_height,
				author_pub,
				price,
			} = data;
			let old_sequence_number = SequenceNumber::<T>::get();
			let order = OrderMap::<T>::get(old_sequence_number);
			// relay chian block number
			let relay_chian_height = T::RelayChainStateProvider::current_relay_chain_state().number;
			let block_number = frame_system::Pallet::<T>::block_number();
			let author = author_pub.expect("author must exist");
			let mut r = [0u8; 32];
			r.copy_from_slice(author.encode().as_slice());
			let orderer = T::AccountId::try_from(r).map_err(|_| Error::<T>::SlotAuthorError)?;
			// Check if the order is in author slot
			if !Self::check_slot_author(place_order_height, orderer.clone()) {
				Err(Error::<T>::SlotAuthorError)?;
			}
			if order.is_none() {
				OrderMap::<T>::insert(
					old_sequence_number,
					Order::<T::AccountId> {
						sequence_number: old_sequence_number,
						relay_chian_height,
						place_order_height,
						orderer: orderer.clone(),
						price,
					},
				);
				SequenceNumber::<T>::set(old_sequence_number + 1);
				Block2Sequence::<T>::insert(block_number, old_sequence_number);
				Self::deposit_event(Event::OrderCreate {
					sequence_number: old_sequence_number,
					orderer,
				});
			} else {
				Err(Error::<T>::OrderExist)?;
			}
			let total_weight = T::DbWeight::get().reads_writes(2, 1);
			Ok(PostDispatchInfo { actual_weight: Some(total_weight), pays_fee: Pays::No })
		}

		/// Order pallet parameter settings.
		/// Set slot width
		/// It can only be called by accounts with sudo privileges or authorized organization members.
		///
		/// Parameters:
		/// - `slot_width`: The order interval is 2^slotwidth..
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_slot_width())]
		pub fn set_slot_width(origin: OriginFor<T>, slot_width: u32) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			<SlotWidth<T>>::put(slot_width);
			Ok(().into())
		}

		/// Order pallet parameter settings.
		/// Set price limit
		/// It can only be called by accounts with sudo privileges or authorized organization members.
		///
		/// Parameters:
		/// - `price_limit`: The maximum price the user is willing to pay when placing an order.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_price_limit())]
		pub fn set_price_limit(
			origin: OriginFor<T>,
			price_limit: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			<PriceLimit<T>>::put(price_limit);
			Ok(().into())
		}
		/// Order pallet parameter settings.
		/// Set gas threshold
		/// It can only be called by accounts with sudo privileges or authorized organization members.
		///
		/// Parameters:
		/// - `threshold`: Gas threshold that triggers order placement.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_gas_threshold())]
		pub fn set_gas_threshold(
			origin: OriginFor<T>,
			threshold: u128,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			<GasThreshold<T>>::put(threshold);
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T>
where
	T::AccountId: From<[u8; 32]>,
{
	/// Whether the gas threshold for placing an order has been reached.
	///
	/// Parameters:
	/// - `gas_balance`: The total gas.
	pub fn reach_txpool_threshold(gas_balance: BalanceOf<T>, core_price: BalanceOf<T>) -> bool
	where
		BalanceOf<T>: Into<u128>,
	{
		let txpool_threshold = GasThreshold::<T>::get();
		let left = gas_balance.into();
		let right = txpool_threshold * core_price.into() / 100;
		left > right
	}

	fn check_slot_author(relaychian_number: u32, author: T::AccountId) -> bool {
		let authorities = pallet_aura::Authorities::<T>::get();
		let slot_width = Self::slot_width();
		let auth_len = authorities.len() as u32;
		// The larger the slot width, the longer the rotation time.
		let idx = (relaychian_number >> slot_width) % auth_len;
		let expected_author = authorities.get(idx as usize);
		log::info!("idx:{:?}, expected_author:{:?}", idx, expected_author);
		if let Some(exp_author) = expected_author {
			if *exp_author.encode() == author.encode() {
				true
			} else {
				false
			}
		} else {
			false
		}
	}
}
