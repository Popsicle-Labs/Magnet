#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

extern crate alloc;

use alloc::{string::ToString, vec, vec::Vec};
use core::marker::PhantomData;
use core::str::from_utf8;
use fp_evm::{ExitError, ExitRevert, ExitSucceed, Precompile, PrecompileFailure};
use fp_evm::{PrecompileHandle, PrecompileOutput, PrecompileResult};
use frame_support::__private::log;
use frame_support::traits::fungibles::Mutate;
use precompile_utils::prelude::*;
use sp_core::H160;
use sp_core::{
	crypto::{AccountId32, Ss58Codec},
	Get, U256,
};
use sp_runtime::traits::UniqueSaturatedInto;

struct MaxSize;
impl Get<u32> for MaxSize {
	fn get() -> u32 {
		256u32
	}
}

pub struct TransferToMagnet<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for TransferToMagnet<T>
where
	T: pallet_evm::Config
		+ pallet_assets_bridge::Config
		+ pallet_assets::Config<AssetIdParameter = codec::Compact<u32>>,
	U256: UniqueSaturatedInto<pallet_evm::BalanceOf<T>>,
	T::AccountId: From<AccountId32>,
	T::AssetId: From<u32> + Into<u32>,
	<T as pallet_assets::Config>::Balance: From<u128>,
{
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let selector = handle.read_u32_selector()?;

		let selector_bytes: [u8; 4] =
			sp_io::hashing::keccak_256(b"transferToMagnet(address,uint256,string)")[0..4]
				.try_into()
				.map_err(|_| PrecompileFailure::Error {
					exit_status: ExitError::Other(
						"get transferToMagnet selector_bytes failed".into(),
					),
				})?;
		let transfer_selector = u32::from_be_bytes(selector_bytes);

		match selector {
			a if a == transfer_selector => _ = Self::transfer_to_magnet(handle)?,
			_ => {
				return Err(PrecompileFailure::Revert {
					exit_status: ExitRevert::Reverted,
					output: "Not find the selector error".into(),
				})
			},
		}

		Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: Vec::new() })
	}
}

impl<T> TransferToMagnet<T>
where
	T: pallet_evm::Config
		+ pallet_assets_bridge::Config
		+ pallet_assets::Config<AssetIdParameter = codec::Compact<u32>>,
	U256: UniqueSaturatedInto<pallet_evm::BalanceOf<T>>,
	T::AccountId: From<AccountId32>,
	T::AssetId: From<u32> + Into<u32>,
	<T as pallet_assets::Config>::Balance: From<u128>,
{
	fn transfer_to_magnet(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		let code_address = handle.code_address();
		let input = handle.input();
		let target_gas = handle.gas_limit();
		let context = handle.context();

		log::debug!(
			"codeAddress:{:?}, input:{:?}, targetGas:{:?}",
			&code_address,
			&input,
			&target_gas
		);
		let caller = context.caller.clone();
		if pallet_assets_bridge::EvmContracts::<T>::get().contains(&caller) == false {
			log::error!("Caller {:?} is not in the admin allow set.", caller);
			//log::error!("EvmContracts:{:?}", pallet_assets_bridge::EvmContracts::<T>::get());
			return Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("Caller is not in the admin allow set".into()),
			});
		}

		let token_addr = solidity::decode_arguments::<Address>(&input[4..36])?;
		let amount = solidity::decode_arguments::<u128>(&input[36..68])?;
		log::debug!("Caller:{:?}, tokenAddr:{:?}, amount:{:?}", &caller, &token_addr, &amount);

		if handle.is_static() {
			log::error!("Can't be static call error");
			return Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("Can't be static call error".into()),
			});
		}

		let to_who_ss58 = match Self::decode_ss58_string(&input, 68) {
			Ok(ss58_address) => ss58_address,
			Err(e) => {
				log::error!("Failed to decode SS58 address: {}", e);
				return Err(PrecompileFailure::Error {
					exit_status: ExitError::Other("Failed to decode SS58 address".into()),
				});
			},
		};
		log::debug!("to who ss58:{:?}", &to_who_ss58);
		let to_who_id32 =
			AccountId32::from_ss58check(&to_who_ss58).map_err(|_| PrecompileFailure::Error {
				exit_status: ExitError::Other("AccountId32 from ss58check(string) failed".into()),
			})?;
		log::debug!("to_who_ss58:{:?}, to_who_id32:{:?}", &to_who_ss58, &to_who_id32);

		let to_who: <T>::AccountId = to_who_id32.clone().into();

		let mut gas_cost: u64 = Self::calculate_gas_cost(&input);
		let log_costs = precompile_utils::evm::costs::log_costs(4, 32)?;

		gas_cost = gas_cost + log_costs;

		if let Some(gas) = target_gas {
			if gas <= gas_cost {
				log::error!("OutOfGas --> targetGas:{:?}, gasCost:{:?}", gas, gas_cost);
				return Err(PrecompileFailure::Error { exit_status: ExitError::OutOfGas });
			}
		}

		handle.record_cost(gas_cost)?;

		let asset_id =
			Self::token_to_asset_id(token_addr.into()).map_err(|_| PrecompileFailure::Error {
				exit_status: ExitError::Other("Failed to get asset_id from token_addr".into()),
			})?;

		let amount_saturated: T::Balance = amount.into();

		log::debug!(
			"Preparing to mint: AssetId: {:?}, Beneficiary: {:?}, Amount: {:?}",
			&asset_id,
			&to_who_ss58,
			&amount_saturated
		);

		let mint_result =
			pallet_assets::Pallet::<T>::mint_into(asset_id.clone(), &to_who, amount_saturated);
		match mint_result {
			Ok(_) => {
				log::info!(
					"Mint operation successful: AssetId: {:?}, Beneficiary: {:?}, Amount: {:?}",
					asset_id,
					to_who,
					amount_saturated
				);
			},
			Err(err) => {
				log::error!("Mint operation failed: AssetId: {:?}, Beneficiary: {:?}, Amount: {:?}, Error: {:?}", asset_id, to_who, amount_saturated, err);
				return Err(PrecompileFailure::Error {
					exit_status: ExitError::Other("mint_into operation failed".into()),
				});
			},
		}

		let event = sp_io::hashing::keccak_256(b"TransferOut(address,string,uint256,string)");
		let ss58_bytes = to_who_ss58.as_bytes();
		let ss58_hash = sp_io::hashing::keccak_256(ss58_bytes);
		let ss58_arg = solidity::encode_arguments::<BoundedString<MaxSize>>(to_who_ss58.into());

		let mut value_bytes: [u8; 32] = [0u8; 32];
		value_bytes[16..].copy_from_slice(&amount.to_be_bytes());

		handle.log(
			code_address,
			vec![event.into(), caller.into(), ss58_hash.into(), value_bytes.into()],
			ss58_arg.into(),
		)?;

		Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output: Vec::new() })
	}

	fn token_to_asset_id(token_addr: H160) -> Result<T::AssetId, PrecompileFailure> {
		pallet_assets_bridge::AssetIds::<T>::get(token_addr).ok_or_else(|| {
			PrecompileFailure::Error {
				exit_status: ExitError::Other("AssetId not found for given token address".into()),
			}
		})
	}

	fn calculate_gas_cost(input: &[u8]) -> u64 {
		const BASE: u64 = 3000 + 20000 + 145572;
		const WORD: u64 = 15;
		BASE + (input.len() as u64 / 32 * WORD)
	}

	#[allow(dead_code)]
	fn ensure_linear_cost(
		target_gas: Option<u64>,
		len: u64,
		base: u64,
		word: u64,
	) -> Result<u64, PrecompileFailure> {
		let cost = base
			.checked_add(
				word.checked_mul(len.saturating_add(31) / 32)
					.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })?,
			)
			.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })?;

		if let Some(target_gas) = target_gas {
			if cost > target_gas {
				return Err(PrecompileFailure::Error { exit_status: ExitError::OutOfGas });
			}
		}

		Ok(cost)
	}

	fn decode_ss58_string(input: &[u8], offset: usize) -> Result<String, &'static str> {
		if input.len() < offset + 32 {
			return Err("Input too short to contain ss58 string length");
		}

		let length_bytes = &input[offset..offset + 32];
		log::debug!("length_bytes:{:?}", &length_bytes);
		let length = u32::from_be_bytes(length_bytes[28..32].try_into().unwrap()) as usize;
		log::debug!("ss58 string len:{:?}", length);

		if input.len() < offset + 32 + length {
			return Err("Input too short to contain string data");
		}

		let string_data_start = offset + 32;
		let string_data_end = string_data_start + length;
		let string_data = &input[string_data_start..string_data_end];
		log::debug!("ss58 string data:{:?}", &string_data);

		let mut result_string = from_utf8(string_data)
			.map_err(|_| "String data is not valid UTF-8")?
			.trim_matches(|c: char| c == ' ' || c == '\0')
			.to_string();

		if result_string.starts_with('0') {
			result_string.remove(0);
		}

		Ok(result_string)
	}
}
