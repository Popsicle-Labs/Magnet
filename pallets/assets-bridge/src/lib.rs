// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # Assets Bridge
//!
//! ## Overview
//!
//! Bridge between pallet-assets and Erc20 tokens

#![cfg_attr(not(feature = "std"), no_std)]

pub mod abi;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub use abi::*;
pub mod recover;
pub use recover::*;

use codec::{Decode, Encode, Input, Output};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{
		tokens::{Fortitude, Precision},
		Currency, IsType,
	},
	transactional,
};

use sp_core::{ecdsa, H160, U256};
use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
use sp_runtime::traits::{StaticLookup, UniqueSaturatedInto, Zero};
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

use pallet_evm::{ExitReason, Runner};

pub type EcdsaSignature = ecdsa::Signature;
pub type AddressMappingOf<T> = <T as pallet_evm::Config>::AddressMapping;
pub type BalanceOf<T> = <<T as pallet_evm::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
pub type ReserveBalanceOf<T> = <<T as pallet_assets::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
pub enum ActionType<AssetId> {
	BackForeign(AssetId),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TransferFlags {
	/// The debited account must stay alive at the end of the operation; an error is returned if
	/// this cannot be achieved legally.
	pub keep_alive: bool,
	/// Less than the amount specified needs be debited by the operation for it to be considered
	/// successful. If `false`, then the amount debited will always be at least the amount
	/// specified.
	pub best_effort: bool,
	/// Any additional funds debited (due to minimum balance requirements) should be burned rather
	/// than credited to the destination account.
	pub burn_dust: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct DebitFlags {
	/// The debited account must stay alive at the end of the operation; an error is returned if
	/// this cannot be achieved legally.
	pub keep_alive: bool,
	/// Less than the amount specified needs be debited by the operation for it to be considered
	/// successful. If `false`, then the amount debited will always be at least the amount
	/// specified.
	pub best_effort: bool,
}

impl From<TransferFlags> for DebitFlags {
	fn from(f: TransferFlags) -> Self {
		Self { keep_alive: f.keep_alive, best_effort: f.best_effort }
	}
}

use scale_info::prelude::*;
use scale_info::TypeInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TypeInfo)]
pub enum LocalPrecision {
	/// The operation should must either proceed either exactly according to the amounts involved
	/// or not at all.
	Exact,
	/// The operation may be considered successful even if less than the specified amounts are
	/// available to be used. In this case a best effort will be made.
	BestEffort,
}

impl Encode for LocalPrecision {
	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		match self {
			LocalPrecision::Exact => dest.push_byte(0),
			LocalPrecision::BestEffort => dest.push_byte(1),
		}
	}
}

impl Decode for LocalPrecision {
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		let byte = u8::decode(input)?;
		match byte {
			0 => Ok(LocalPrecision::Exact),
			1 => Ok(LocalPrecision::BestEffort),
			_ => Err(codec::Error::from("Invalid value for LocalPrecision")),
		}
	}
}

impl From<LocalPrecision> for Precision {
	fn from(local: LocalPrecision) -> Self {
		match local {
			LocalPrecision::Exact => Precision::Exact,
			LocalPrecision::BestEffort => Precision::BestEffort,
		}
	}
}

impl From<Precision> for LocalPrecision {
	fn from(external: Precision) -> Self {
		match external {
			Precision::Exact => LocalPrecision::Exact,
			Precision::BestEffort => LocalPrecision::BestEffort,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TypeInfo)]
pub enum LocalFortitude {
	/// The operation should execute with regular privilege.
	Polite,
	/// The operation should be forced to succeed if possible. This is usually employed for system-
	/// level security-critical events such as slashing.
	Force,
}

impl Encode for LocalFortitude {
	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
		match self {
			LocalFortitude::Polite => dest.push_byte(0),
			LocalFortitude::Force => dest.push_byte(1),
		}
	}
}

impl Decode for LocalFortitude {
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		let byte = u8::decode(input)?;
		match byte {
			0 => Ok(LocalFortitude::Polite),
			1 => Ok(LocalFortitude::Force),
			_ => Err(codec::Error::from("Invalid value for LocalFortitude")),
		}
	}
}

impl From<LocalFortitude> for Fortitude {
	fn from(local: LocalFortitude) -> Self {
		match local {
			LocalFortitude::Polite => Fortitude::Polite,
			LocalFortitude::Force => Fortitude::Force,
		}
	}
}

impl From<Fortitude> for LocalFortitude {
	fn from(external: Fortitude) -> Self {
		match external {
			Fortitude::Polite => LocalFortitude::Polite,
			Fortitude::Force => LocalFortitude::Force,
		}
	}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::fungibles::Mutate;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	//#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config + pallet_evm::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The assets-bridge's inner evm caller.
		#[pallet::constant]
		type EvmCaller: Get<H160>;
		/// How much should be locked up in order to claim account.
		#[pallet::constant]
		type ClaimBond: Get<ReserveBalanceOf<Self>>;
	}

	impl<T: Config> Pallet<T> {
		fn ensure_admin(who: &T::AccountId) -> Result<(), Error<T>> {
			if Some(who.clone()) != Self::admin_key() {
				Err(Error::<T>::RequireAdmin)
			} else {
				Ok(())
			}
		}

		fn is_in_emergency(asset_id: T::AssetId) -> bool {
			Self::emergencies()
				.iter()
				.any(|emergency| emergency.clone() == asset_id.clone())
		}

		fn is_in_back_foreign(asset_id: T::AssetId) -> bool {
			Self::back_foreign_assets().iter().any(|id| id.clone() == asset_id.clone())
		}
	}

	/// The Substrate Account for Evm Addresses
	///
	/// SubAccounts: map H160 => Option<AccountId>
	#[pallet::storage]
	#[pallet::getter(fn sub_accounts)]
	pub type SubAccounts<T: Config> = StorageMap<_, Twox64Concat, H160, T::AccountId, OptionQuery>;

	/// The Evm Addresses for Substrate Accounts
	///
	/// EvmAccounts: map AccountId => Option<H160>
	#[pallet::storage]
	#[pallet::getter(fn evm_accounts)]
	pub type EvmAccounts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, H160, OptionQuery>;

	/// The Erc20 Contract Addresses for Asset Ids
	///
	/// Erc20s: map AssetId => Option<H160>
	#[pallet::storage]
	#[pallet::getter(fn erc20s)]
	pub type Erc20s<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, H160, OptionQuery>;

	/// The Asset Ids for Erc20 Contract Addresses
	///
	/// AssetIds: map H160 => Option<AssetId>
	#[pallet::storage]
	#[pallet::getter(fn asset_ids)]
	pub type AssetIds<T: Config> = StorageMap<_, Twox64Concat, H160, T::AssetId, OptionQuery>;

	/// The Assets can back foreign chain
	///
	/// AssetIds: Vec<AssetId>
	#[pallet::storage]
	#[pallet::getter(fn back_foreign_assets)]
	pub type BackForeign<T: Config> = StorageValue<_, Vec<T::AssetId>, ValueQuery>;

	/// The pallet admin key.
	#[pallet::storage]
	#[pallet::getter(fn admin_key)]
	pub(super) type Admin<T: Config> = StorageValue<_, T::AccountId>;

	/// The Assets in emergency
	#[pallet::storage]
	#[pallet::getter(fn emergencies)]
	pub(super) type Emergencies<T: Config> = StorageValue<_, Vec<T::AssetId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn evm_contracts)]
	pub type EvmContracts<T: Config> = StorageValue<_, BTreeSet<H160>, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// The `AccountId` of the admin key.
		pub admin_key: Option<T::AccountId>,
	}

	/*
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { admin_key: Default::default() }
		}
	}
	*/

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(key) = &self.admin_key {
				<Admin<T>>::put(key.clone());
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// (account_id, evm_address)
		ClaimAccount(T::AccountId, H160),
		/// (account_id)
		Dissolve(T::AccountId),
		/// (asset_id, account_id, evm_address, amount, erc20_contract)
		DepositExecuted(T::AssetId, T::AccountId, H160, T::Balance, H160),
		/// (asset_id, account_id, evm_address, amount, erc20_contract)
		WithdrawExecuted(T::AssetId, T::AccountId, H160, T::Balance, H160),
		/// (account_id, amount, action)
		Teleport(T::AccountId, BalanceOf<T>, ActionType<T::AssetId>),
		/// (account_id)
		SetAdmin(T::AccountId),
		/// (asset_id, erc20_contract)
		Register(T::AssetId, H160),
		/// (asset_id, erc20_contract)
		ForceUnRegister(T::AssetId, H160),
		/// (asset_id)
		Paused(T::AssetId),
		// (asset_id)
		UnPaused(T::AssetId),
		PausedAll,
		UnPausedAll,
		// (asset_id, remove)
		BackForeign(T::AssetId, bool),

		///(new_evm_contract)
		AddNewContract(H160),
		///(evm contract)
		RemoveContract(H160),
	}

	/// Error for evm accounts module.
	#[pallet::error]
	pub enum Error<T> {
		/// AccountId has mapped
		AccountIdHasMapped,
		/// Eth address has mapped
		EthAddressHasMapped,
		/// Bad signature
		BadSignature,
		/// Invalid signature
		InvalidSignature,
		/// AccountId has not mapped
		AccountIdHasNotMapped,
		/// Eth address has not mapped
		EthAddressHasNotMapped,
		/// AssetId has mapped
		AssetIdHasMapped,
		/// AssetId has not mapped
		AssetIdHasNotMapped,
		/// Erc20 contract address has mapped
		ContractAddressHasMapped,
		/// Erc20 contract address has not mapped
		ContractAddressHasNotMapped,
		/// Failed Erc20 contract call
		ExecutedFailed,
		/// Require admin authority
		RequireAdmin,
		/// Ban deposit and withdraw when in emergency
		InEmergency,
		/// Ban back to foreign
		BanBackForeign,
		/// Zero balance
		ZeroBalance,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		DispatchError: From<<<T as pallet_evm::Config>::Runner as pallet_evm::Runner<T>>::Error>,
	{
		/// Deposit substrate assets into evm erc20 contracts.
		/// Note: for general users
		///
		/// - `asset_id`: The asset id
		/// - `amount`: Deposit amount
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		#[transactional]
		pub fn deposit(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			evm_account: H160,
			amount: T::Balance,
			precision: LocalPrecision,
			force: LocalFortitude,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(!Self::is_in_emergency(asset_id.clone()), Error::<T>::InEmergency);
			ensure!(!amount.is_zero(), Error::<T>::ZeroBalance);

			// 1. check evm account
			//let evm_account = Self::evm_accounts(&who).ok_or(Error::<T>::EthAddressHasNotMapped)?;

			let external_precision: Precision = precision.into();
			let external_force: Fortitude = force.into();
			// 2. burn asset
			let _ = pallet_assets::Pallet::<T>::burn_from(
				asset_id.clone(),
				&who,
				amount,
				external_precision,
				external_force,
			)?;

			// 3. mint erc20
			let erc20 =
				Self::erc20s(asset_id.clone()).ok_or(Error::<T>::ContractAddressHasNotMapped)?;

			let inputs = mint_into_encode(evm_account, amount.unique_saturated_into());

			Self::call_evm(erc20, inputs)?;

			Self::deposit_event(Event::DepositExecuted(asset_id, who, evm_account, amount, erc20));

			Ok(Pays::No.into())
		}

		/// Teleport native currency between substrate account and evm address
		/// Note: for general users
		///
		/// - `amount`: Teleport amount
		/// - `action`:
		/// - (1) BackForeign(asset_id): transfer assets back foreign chain
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		#[transactional]
		pub fn teleport(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			action: ActionType<T::AssetId>,
			precision: LocalPrecision,
			force: LocalFortitude,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let action_clone = action.clone();

			ensure!(!amount.is_zero(), Error::<T>::ZeroBalance);

			let (_from, _to, _back_foreign) = match action {
				ActionType::BackForeign(asset_id) => {
					// ensure asset_id registered in back_foreign list
					ensure!(Self::is_in_back_foreign(asset_id.clone()), Error::<T>::BanBackForeign);
					ensure!(!Self::is_in_emergency(asset_id.clone()), Error::<T>::InEmergency);

					let amount: u128 = amount.unique_saturated_into();
					// burn asset first, then relay will transfer back `who`.
					let external_precision: Precision = precision.into();
					let external_force: Fortitude = force.into();

					let _ = pallet_assets::Pallet::<T>::burn_from(
						asset_id.clone(),
						&who,
						amount.unique_saturated_into(),
						external_precision,
						external_force,
					)?;

					(who.clone(), who.clone(), true)
				},
			};

			Self::deposit_event(Event::Teleport(who, amount, action_clone));

			Ok(Pays::No.into())
		}

		/// Register substrate assets and erc20 contracts
		/// Note: for admin
		///
		/// - `asset_id`: The asset id
		/// - `erc20`: The erc20 contract address
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn register(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			erc20: H160,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::ensure_admin(&who)?;

			// ensure asset_id and erc20 address has not been mapped
			ensure!(!Erc20s::<T>::contains_key(asset_id.clone()), Error::<T>::AssetIdHasMapped);
			ensure!(!AssetIds::<T>::contains_key(erc20), Error::<T>::ContractAddressHasMapped);

			Erc20s::<T>::insert(asset_id.clone(), erc20);
			AssetIds::<T>::insert(erc20, asset_id.clone());

			Self::deposit_event(Event::Register(asset_id.clone(), erc20));

			Ok(Pays::No.into())
		}

		/// Pause assets bridge deposit and withdraw
		/// Note: for admin
		///
		/// - `asset_id`: None will pause all, Some(id) will pause the specified asset
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn pause(
			origin: OriginFor<T>,
			asset_id: Option<T::AssetId>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::ensure_admin(&who)?;

			Emergencies::<T>::try_mutate(|emergencies| {
				if let Some(id) = asset_id.clone() {
					// ensure asset_id and erc20 address has not been mapped
					ensure!(Erc20s::<T>::contains_key(id.clone()), Error::<T>::AssetIdHasNotMapped);
					if !Self::is_in_emergency(id.clone()) {
						emergencies.push(id.clone());

						Self::deposit_event(Event::Paused(id.clone()));
					}
				} else {
					emergencies.truncate(0);
					for id in AssetIds::<T>::iter_values() {
						emergencies.push(id);
					}

					Self::deposit_event(Event::PausedAll);
				}

				Ok(Pays::No.into())
			})
		}

		/// Unpause assets bridge deposit and withdraw
		/// Note: for admin
		///
		/// - `asset_id`: None will unpause all, Some(id) will unpause the specified asset
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn unpause(
			origin: OriginFor<T>,
			asset_id: Option<T::AssetId>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

			Emergencies::<T>::try_mutate(|emergencies| {
				if let Some(id) = asset_id.clone() {
					// ensure asset_id and erc20 address has been mapped
					ensure!(Erc20s::<T>::contains_key(id.clone()), Error::<T>::AssetIdHasNotMapped);

					if Self::is_in_emergency(id.clone()) {
						emergencies.retain(|emergency| emergency.clone() != id.clone());

						Self::deposit_event(Event::UnPaused(id.clone()));
					}
				} else {
					emergencies.truncate(0);

					Self::deposit_event(Event::UnPausedAll);
				}

				Ok(Pays::No.into())
			})
		}

		/// Add assets which can back add_back_foreign chain
		/// Note: for admin
		///
		/// - `asset_id`:
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn back_foreign(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			remove: bool,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

			BackForeign::<T>::try_mutate(|foreigns| {
				if remove {
					foreigns.retain(|id| *id != asset_id.clone());
				} else if !Self::is_in_back_foreign(asset_id.clone()) {
					foreigns.push(asset_id.clone());
				} else {
					return Ok(Pays::No.into());
				}

				Self::deposit_event(Event::BackForeign(asset_id.clone(), remove));

				Ok(Pays::No.into())
			})
		}

		/// Set this pallet admin key
		/// Note: for super admin
		#[pallet::call_index(6)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn set_admin(
			origin: OriginFor<T>,
			new_admin: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResultWithPostInfo {
			let require = match ensure_signed_or_root(origin) {
				Ok(s) if s == Self::admin_key() => true,
				Ok(None) => true,
				_ => false,
			};

			ensure!(require, Error::<T>::RequireAdmin);

			let new_admin = T::Lookup::lookup(new_admin)?;

			Admin::<T>::mutate(|admin| *admin = Some(new_admin.clone()));

			Self::deposit_event(Event::SetAdmin(new_admin));

			Ok(Pays::No.into())
		}

		/// Force unregister substrate assets and erc20 contracts
		/// Note: for super admin
		#[pallet::call_index(7)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn force_unregister(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let erc20 = Self::erc20s(asset_id.clone()).ok_or(Error::<T>::AssetIdHasNotMapped)?;

			ensure!(AssetIds::<T>::contains_key(erc20), Error::<T>::ContractAddressHasMapped);

			Erc20s::<T>::remove(asset_id.clone());
			AssetIds::<T>::remove(erc20);

			// clear emergency
			if Self::is_in_emergency(asset_id.clone()) {
				Emergencies::<T>::mutate(|emergencies| {
					emergencies.retain(|emergency| emergency.clone() != asset_id.clone());
				})
			}

			Self::deposit_event(Event::ForceUnRegister(asset_id.clone(), erc20));

			Ok(Pays::No.into())
		}

		/// Add evm token contracts which can call precompile
		/// Note: for admin
		///
		/// - `new_contract`:
		#[pallet::call_index(8)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn add_evm_contract(
			origin: OriginFor<T>,
			new_contract: H160,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::ensure_admin(&who)?;

			EvmContracts::<T>::mutate(|contracts| {
				contracts.insert(new_contract);
			});

			Self::deposit_event(Event::AddNewContract(new_contract.clone()));

			Ok(Pays::No.into())
		}

		/// Remove evm token contracts which can call precompile
		/// Note: for admin
		///
		/// - `new_contract`:
		#[pallet::call_index(9)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn remove_evm_contract(
			origin: OriginFor<T>,
			contract: H160,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Some(who) == Self::admin_key(), Error::<T>::RequireAdmin);

			EvmContracts::<T>::mutate(|contracts| {
				contracts.remove(&contract);
			});

			Self::deposit_event(Event::RemoveContract(contract));

			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T>
where
	DispatchError: From<<<T as pallet_evm::Config>::Runner as pallet_evm::Runner<T>>::Error>,
{
	fn call_evm(erc20: H160, inputs: Vec<u8>) -> DispatchResult {
		let info = T::Runner::call(
			T::EvmCaller::get(),
			erc20,
			inputs,
			U256::default(),
			3_000_000,
			None,
			None,
			None,
			Vec::new(),
			false,
			true,
			None,
			None,
			T::config(),
		)
		.map_err(|e| e.error.into())?;

		match info.exit_reason {
			ExitReason::Succeed(_) => Ok(()),
			_ => Err(Error::<T>::ExecutedFailed.into()),
		}
	}
}
