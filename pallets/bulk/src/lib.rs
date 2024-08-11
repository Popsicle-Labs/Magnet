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

//! # Bulk Pallet
//!
//! This pallet implements the recording and query functions of purchasing bulk core.
//!

#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelaychainStateProvider;
use cumulus_primitives_core::relay_chain::Hash as PHash;
use frame_support::{
	dispatch::DispatchResultWithPostInfo, dispatch::PostDispatchInfo, pallet_prelude::*,
	traits::Currency,
};
use frame_system::pallet_prelude::*;
use mp_coretime_bulk::{well_known_keys::broker_regions, RegionRecord, RegionRecordV0};
use mp_coretime_common::{
	chain_state_snapshot::GenericStateProof, well_known_keys::SYSTEM_BLOCKHASH_GENESIS,
};
pub use pallet::*;
use sp_runtime::sp_std::{prelude::*, vec};
use weights::WeightInfo;

#[cfg(test)]
mod mock;
pub mod weights;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod benchmarking;
mod proof_data;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Purchase coretime information.
#[derive(Encode, Decode, Default, Clone, Copy, TypeInfo, MaxEncodedLen, Debug)]
pub struct BulkRecord<Balance, AuthorityId> {
	/// Account for purchase.
	pub purchaser: AuthorityId,
	/// Purchase price.
	pub price: Balance,
	/// Purchase duration.
	pub duration: u32,
	/// Relaychain block number of start schedule coretime core.
	pub start_relaychain_height: u32,
	/// Relaychain block number of end schedule coretime core.
	pub end_relaychain_height: u32,
	/// Relaychain block number of parachain start run.
	pub real_start_relaychain_height: u32,
	/// Relaychain block number of parachain end run.
	pub real_end_relaychain_height: u32,
}
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: Currency<Self::AccountId>;

		type RelayChainStateProvider: cumulus_pallet_parachain_system::RelaychainStateProvider;

		type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type WeightInfo: WeightInfo;

		/// Max length of url, coretime parachain rpc url.
		#[pallet::constant]
		type MaxUrlLength: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	/// Url storage.
	#[pallet::storage]
	#[pallet::getter(fn coretime_rpc_url)]
	pub type RpcUrl<T: Config> = StorageValue<_, BoundedVec<u8, T::MaxUrlLength>, OptionQuery>;

	#[pallet::type_value]
	pub fn RecordIndexOnEmpty<T: Config>() -> u32 {
		0
	}

	/// The index of the record, plus one for each additional record.
	#[pallet::storage]
	#[pallet::getter(fn record_index)]
	pub type RecordIndex<T> = StorageValue<_, u32, ValueQuery, RecordIndexOnEmpty<T>>;

	/// Coretime parachain genesis block hash,for check.
	#[pallet::storage]
	#[pallet::getter(fn genesis_hash)]
	pub type GenesisHash<T> = StorageValue<_, PHash, ValueQuery>;

	#[pallet::type_value]
	pub fn CheckGenesisHashOnEmpty<T: Config>() -> bool {
		false
	}

	/// Is check coretime parachain genesis block hash or not.
	#[pallet::storage]
	#[pallet::getter(fn check_genesis_hash)]
	pub type CheckGenesisHash<T> = StorageValue<_, bool, ValueQuery, CheckGenesisHashOnEmpty<T>>;

	/// Bulk purchase Information Map.
	#[pallet::storage]
	#[pallet::getter(fn bulk_records)]
	pub type BulkRecords<T: Config> =
		StorageMap<_, Twox64Concat, u32, BulkRecord<BalanceOf<T>, T::AccountId>, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub rpc_url: BoundedVec<u8, T::MaxUrlLength>,
		pub genesis_hash: PHash,
		pub check_genesis_hash: bool,
		pub _marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				rpc_url: BoundedVec::new(),
				genesis_hash: Default::default(),
				check_genesis_hash: false,
				_marker: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			RpcUrl::<T>::put(&self.rpc_url);
			GenesisHash::<T>::put(&self.genesis_hash);
			CheckGenesisHash::<T>::put(&self.check_genesis_hash);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Create record event.
		RecordCreated {
			/// Account for purchase.
			purchaser: T::AccountId,
			/// Purchase price.
			price: BalanceOf<T>,
			/// Purchase duration.
			duration: u32,
			/// Relaychain block number of start schedule coretime core.
			start_relaychain_height: u32,
			/// Relaychain block number of end schedule coretime core.
			end_relaychain_height: u32,
			/// Relaychain block number of parachain start run.
			real_start_relaychain_height: u32,
			/// Relaychain block number of parachain end run.
			real_end_relaychain_height: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error reading data.
		FailedReading,
		/// Storage proof is none.
		ProofNone,
		/// Create root proof failed.
		FailedCreateProof,
		/// Purchaser is none.
		PurchaserNone,
		/// Genesis hash inconsistency
		GenesisHashInconsistency,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = MakeFatalError<()>;

		const INHERENT_IDENTIFIER: InherentIdentifier = mp_coretime_bulk::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let data: Option<mp_coretime_bulk::BulkInherentData> =
				data.get_data(&mp_coretime_bulk::INHERENT_IDENTIFIER).ok().flatten();
			match data {
				Some(data) => {
					if data.storage_proof.is_none() {
						None
					} else {
						Some(Call::create_record { data })
					}
				},
				None => None,
			}
		}
		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::create_record { .. })
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create an bulk record, which is called by the pallet.
		/// Users cannot actively call this function.
		/// Obtain record information by parsing inherited data.
		///
		/// Parameters:
		/// - `data`: The inherent data.
		#[pallet::call_index(0)]
		#[pallet::weight((<T as pallet::Config>::WeightInfo::create_record(), DispatchClass::Mandatory))]
		pub fn create_record(
			origin: OriginFor<T>,
			data: mp_coretime_bulk::BulkInherentData,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			let mp_coretime_bulk::BulkInherentData {
				storage_proof: p_storage_proof,
				storage_root,
				region_id,
				duration,
				start_relaychain_height,
				end_relaychain_height,
			} = data;

			let storage_proof = p_storage_proof.ok_or(Error::<T>::ProofNone)?;
			// Create coretime parachain root proof
			let storage_rooted_proof: GenericStateProof<
				cumulus_primitives_core::relay_chain::Block,
			> = GenericStateProof::new(storage_root, storage_proof)
				.map_err(|_| Error::<T>::FailedCreateProof)?;

			let region_key = broker_regions(region_id);
			// Read RegionRecord from proof.
			let p_region_record = storage_rooted_proof
				.read_entry::<RegionRecord<T::AccountId, BalanceOf<T>>>(region_key.as_slice(), None)
				.ok();
			let (balance, purchaser) = if let Some(region_record) = p_region_record {
				let balance = region_record.paid.ok_or(Error::<T>::PurchaserNone)?;
				let purchaser = region_record.owner.ok_or(Error::<T>::PurchaserNone)?;
				(balance, purchaser)
			} else {
				let region_record = storage_rooted_proof
					.read_entry::<RegionRecordV0<T::AccountId, BalanceOf<T>>>(
						region_key.as_slice(),
						None,
					)
					.ok()
					.ok_or(Error::<T>::FailedReading)?;
				let balance = region_record.paid.ok_or(Error::<T>::PurchaserNone)?;
				let purchaser = region_record.owner;
				(balance, purchaser)
			};

			if Self::check_genesis_hash() {
				let genesis_hash_key = SYSTEM_BLOCKHASH_GENESIS;
				let genesis_hash = storage_rooted_proof
					.read_entry::<PHash>(genesis_hash_key, None)
					.ok()
					.ok_or(Error::<T>::FailedReading)?;

				let stored_hash = GenesisHash::<T>::get();
				if genesis_hash != stored_hash {
					Err(Error::<T>::GenesisHashInconsistency)?;
				}
			}

			let real_start_relaychain_height = Self::relaychain_block_number();
			let real_end_relaychain_height = real_start_relaychain_height + duration;
			let old_record_index = RecordIndex::<T>::get();

			// Create record of purchase coretime.
			BulkRecords::<T>::insert(
				old_record_index,
				BulkRecord::<BalanceOf<T>, T::AccountId> {
					purchaser: purchaser.clone(),
					price: balance,
					duration,
					start_relaychain_height,
					end_relaychain_height,
					real_start_relaychain_height,
					real_end_relaychain_height,
				},
			);
			RecordIndex::<T>::set(old_record_index + 1);
			Self::deposit_event(Event::RecordCreated {
				purchaser,
				price: balance,
				duration,
				start_relaychain_height,
				end_relaychain_height,
				real_start_relaychain_height,
				real_end_relaychain_height,
			});

			let total_weight = T::WeightInfo::create_record();
			Ok(PostDispatchInfo { actual_weight: Some(total_weight), pays_fee: Pays::No })
		}

		/// Set coretime parachain rpc url.
		///
		/// Parameters:
		/// - `url`: Url.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_rpc_url())]
		pub fn set_rpc_url(
			origin: OriginFor<T>,
			url: BoundedVec<u8, T::MaxUrlLength>,
		) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			RpcUrl::<T>::put(url.clone());

			Ok(())
		}

		/// Set coretime parachain genesis hash.
		///
		/// Parameters:
		/// - `genesis_hash`: genesis hash.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_genesis_hash())]
		pub fn set_genesis_hash(origin: OriginFor<T>, genesis_hash: PHash) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			GenesisHash::<T>::put(genesis_hash);

			Ok(())
		}

		/// Set check coretime parachain genesis hash.
		///
		/// Parameters:
		/// - `check`: check genesis hash.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_check_genesis_hash())]
		pub fn set_check_genesis_hash(origin: OriginFor<T>, check: bool) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			CheckGenesisHash::<T>::put(check);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Get coretime parachain rpc url.
	pub fn rpc_url() -> Vec<u8> {
		let rpc_url = RpcUrl::<T>::get();
		if let Some(url) = rpc_url {
			url.into()
		} else {
			Vec::new()
		}
	}

	/// Get relaychain blocknumber.
	pub fn relaychain_block_number() -> u32 {
		let relay_chain_state = T::RelayChainStateProvider::current_relay_chain_state();
		relay_chain_state.number
	}
}
