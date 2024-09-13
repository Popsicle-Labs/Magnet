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

//! Coretime common function of client.
//!
pub mod configuration;
pub mod types;

use crate::types::QueueStatusType;
use codec::Decode;
use configuration::{
	HostConfiguration, V10HostConfiguration, V11HostConfiguration, V8HostConfiguration,
	V9HostConfiguration,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_interface::RelayChainInterface;
use mp_coretime_common::well_known_keys::{
	paras_para_lifecycles, ACTIVE_CONFIG, CONFIGURATION_VERSION, ON_DEMAND_VERSION,
	QUEUE_STATUS_TYPE, SPOT_TRAFFIC,
};
use runtime_parachains::paras::ParaLifecycle;
use sp_core::{
	crypto::{ByteArray, Pair},
	H256,
};
use sp_keystore::KeystorePtr;
use sp_runtime::{traits::SaturatedConversion, FixedPointNumber, FixedU128};
use std::error::Error;

type AuthorityId<P> = <P as Pair>::Public;

/// Is it now a parathread.
pub async fn is_parathread(
	relay_chain: &(impl RelayChainInterface + Clone),
	p_hash: H256,
	para_id: ParaId,
) -> Result<bool, Box<dyn Error>> {
	let para_lifecycles_storage = relay_chain
		.get_storage_by_key(p_hash, paras_para_lifecycles(para_id).as_slice())
		.await?;
	let para_lifecycles = para_lifecycles_storage
		.map(|raw| <ParaLifecycle>::decode(&mut &raw[..]))
		.transpose()?;
	let is_parathread = match para_lifecycles {
		Some(lifecycles) => matches!(
			lifecycles,
			ParaLifecycle::Parathread
				| ParaLifecycle::UpgradingParathread
				| ParaLifecycle::OffboardingParathread
		),
		None => false,
	};
	Ok(is_parathread)
}

pub async fn coretime_cores(
	relay_chain: &(impl RelayChainInterface + Clone),
	p_hash: H256,
) -> Option<u32> {
	// pallet version
	let pallet_version_storage =
		relay_chain.get_storage_by_key(p_hash, CONFIGURATION_VERSION).await.ok()?;
	let pallet_version = pallet_version_storage
		.map(|raw| <u16>::decode(&mut &raw[..]))
		.transpose()
		.ok()??;
	let active_config_storage = relay_chain.get_storage_by_key(p_hash, ACTIVE_CONFIG).await.ok()?;
	// Get cores
	let cores = if pallet_version == 8 {
		let configuration = active_config_storage
			.map(|raw| <V8HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.on_demand_cores
	} else if pallet_version == 9 {
		let configuration = active_config_storage
			.map(|raw| <V9HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.on_demand_cores
	} else if pallet_version == 10 {
		let configuration = active_config_storage
			.map(|raw| <V10HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.on_demand_cores
	} else if pallet_version == 11 {
		let configuration = active_config_storage
			.map(|raw| <V11HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.coretime_cores
	} else {
		let configuration = active_config_storage
			.map(|raw| <HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.scheduler_params.num_cores
	};

	Some(cores)
}

pub async fn relaychain_spot_price(
	relay_chain: &(impl RelayChainInterface + Clone),
	p_hash: H256,
) -> Option<u128> {
	// configuration pallet version
	let ondemand_version_storage =
		relay_chain.get_storage_by_key(p_hash, ON_DEMAND_VERSION).await.ok()?;
	let ondemand_version = ondemand_version_storage
		.map(|raw| <u16>::decode(&mut &raw[..]))
		.transpose()
		.ok()??;
	let spot_traffic = if ondemand_version == 0 {
		let spot_traffic_storage =
			relay_chain.get_storage_by_key(p_hash, SPOT_TRAFFIC).await.ok()?;
		let traffic = spot_traffic_storage
			.map(|raw| <FixedU128>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		traffic
	} else {
		let spot_traffic_storage =
			relay_chain.get_storage_by_key(p_hash, QUEUE_STATUS_TYPE).await.ok()?;
		let traffic = spot_traffic_storage
			.map(|raw| <QueueStatusType>::decode(&mut &raw[..]))
			.transpose()
			.ok()??
			.traffic;
		FixedU128::from_rational(traffic.0, FixedU128::accuracy())
	};
	// configuration pallet version
	let configuration_version_storage =
		relay_chain.get_storage_by_key(p_hash, CONFIGURATION_VERSION).await.ok()?;
	let configuration_version = configuration_version_storage
		.map(|raw| <u16>::decode(&mut &raw[..]))
		.transpose()
		.ok()??;
	let active_config_storage = relay_chain.get_storage_by_key(p_hash, ACTIVE_CONFIG).await.ok()?;
	// Get cores
	let on_demand_base_fee = if configuration_version == 8 {
		let configuration = active_config_storage
			.map(|raw| <V8HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.on_demand_base_fee
	} else if configuration_version == 9 {
		let configuration = active_config_storage
			.map(|raw| <V9HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.on_demand_base_fee
	} else if configuration_version == 10 {
		let configuration = active_config_storage
			.map(|raw| <V10HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.on_demand_base_fee
	} else if configuration_version == 11 {
		let configuration = active_config_storage
			.map(|raw| <V11HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.on_demand_base_fee
	} else {
		let configuration = active_config_storage
			.map(|raw| <HostConfiguration<u32>>::decode(&mut &raw[..]))
			.transpose()
			.ok()??;
		configuration.scheduler_params.on_demand_base_fee
	};
	let spot_price = spot_traffic.saturating_mul_int(on_demand_base_fee.saturated_into::<u128>());
	Some(spot_price)
}

/// Randomly select a collator to place an order.
pub async fn order_slot<P: Pair>(
	idx: u32,
	authorities: &[AuthorityId<P>],
	keystore: &KeystorePtr,
) -> Option<P::Public> {
	if authorities.is_empty() {
		return None;
	}

	let expected_author = authorities.get(idx as usize).expect(
		"authorities not empty; index constrained to list length;this is a valid index; qed",
	);

	if keystore.has_keys(&[(expected_author.to_raw_vec(), sp_application_crypto::key_types::AURA)])
	{
		Some(expected_author.clone())
	} else {
		None
	}
}

/// Get the collator of this slot.
pub async fn order_slot_author<P: Pair>(
	idx: u32,
	authorities: &[AuthorityId<P>],
) -> Option<P::Public> {
	if authorities.is_empty() {
		return None;
	}

	let expected_author = authorities.get(idx as usize).expect(
		"authorities not empty; index constrained to list length;this is a valid index; qed",
	);

	Some(expected_author.clone())
}
