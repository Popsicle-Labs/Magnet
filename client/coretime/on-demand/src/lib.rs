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

//! Ondemand order Spawner
//!
//! The logic of placing an order to purchase core is as follows.
//! Every time a relay chain block is received, check whether the parachain is in parallel thread mode.
//! If so, read the events of the relay chain to see if there is an order to purchase core. , if there is a purchase,
//! record the purchase information. If not, then determine whether it is necessary to purchase core, such as whether
//! the transactions in the mempool reach a certain threshold.
//!
mod metadata;
mod submit_order;

use codec::{Codec, Decode};
use cumulus_primitives_core::{
	relay_chain::BlockNumber as RelayBlockNumber, ParaId, PersistedValidationData,
};
use cumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use futures::{lock::Mutex, pin_mut, select, FutureExt, Stream, StreamExt};
use mc_coretime_common::{coretime_cores, is_parathread, relaychain_spot_price};
use metadata::{CoreAssignment, CoreDescriptor};
use mp_coretime_on_demand::{
	self, well_known_keys::paras_core_descriptors, OrderRecord, OrderRuntimeApi, OrderStatus,
};
use mp_system::OnRelayChainApi;
pub use pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi;
use polkadot_primitives::OccupiedCoreAssumption;
use sc_client_api::UsageProvider;
use sc_service::TaskManager;
use sc_transaction_pool_api::{InPoolTransaction, MaintainedTransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_consensus_aura::sr25519::AuthorityId;
use sp_consensus_aura::sr25519::AuthorityPair;
use sp_consensus_aura::AuraApi;
use sp_core::H256;
use sp_keystore::KeystorePtr;
use sp_runtime::{
	codec::Encode,
	traits::{AtLeast32BitUnsigned, Block as BlockT, Header as HeaderT, MaybeDisplay},
};
use std::{cmp::Ordering, net::SocketAddr};
use std::{error::Error, fmt::Debug, sync::Arc};
use submit_order::{build_rpc_for_submit_order, SubmitOrderError};
use subxt::backend::{legacy::LegacyRpcMethods, rpc::RpcClient};
use subxt::{OnlineClient, PolkadotConfig};

/// Order type
#[derive(Clone, PartialEq, Debug)]
pub enum OrderType {
	/// The mem pool gas reaches the threshold.
	Normal,
	/// Reaching the forced block threshold.
	Force,
	/// Receive xcm transaction from relay chain.
	XCMEvent,
}

/// Get the spot price of the relay chain.
async fn get_spot_price<Balance>(
	relay_chain: impl RelayChainInterface + Clone,
	hash: H256,
) -> Option<Balance>
where
	Balance: Codec + MaybeDisplay + 'static + Debug + From<u128>,
{
	let p_spot_price = relaychain_spot_price(&relay_chain, hash).await;
	log::info!("=============p_spot_price:{:?}", p_spot_price);
	if let Some(spot_price) = p_spot_price {
		Some(Balance::from(spot_price))
	} else {
		None
	}
}

/// Whether the relay chain has ondemand function enabled.
async fn start_on_demand(
	relay_chain: impl RelayChainInterface + Clone + Send,
	hash: H256,
	para_id: ParaId,
) -> Option<bool> {
	// Get the number of cores.
	let r_cores = coretime_cores(&relay_chain, hash).await;
	let mut result = false;
	if let Some(cores) = r_cores {
		for core in 0..cores {
			let key = paras_core_descriptors(polkadot_primitives::CoreIndex(core));
			let core_descriptors_storage =
				relay_chain.get_storage_by_key(hash, key.as_slice()).await.ok()?;
			let p_core_descriptors = core_descriptors_storage
				.map(|raw| <CoreDescriptor<u32>>::decode(&mut &raw[..]))
				.transpose()
				.ok()?;
			if let Some(core_descriptors) = p_core_descriptors {
				let p_current_work = core_descriptors.current_work;
				if let Some(current_work) = p_current_work {
					for (assign, _) in current_work.assignments {
						if assign == CoreAssignment::Task(para_id.into()) {
							return Some(false); // para chain bulk mode.
						} else if assign == CoreAssignment::Pool {
							// find on demand core
							result = true
						}
					}
				}
			}
		}
	}
	Some(result)
}

/// Create an order to purchase core.
async fn try_place_order<Balance>(
	keystore: KeystorePtr,
	para_id: ParaId,
	url: String,
	max_amount: Balance,
) -> Result<(), SubmitOrderError>
where
	Balance: Codec + MaybeDisplay + 'static + Debug + Into<u128>,
	ParaId: From<u32>,
{
	let max_amount_128 = max_amount.into();
	build_rpc_for_submit_order(&url, para_id, max_amount_128, keystore).await
}

/// Whether the mem pool reaches the threshold for purchasing cores.
async fn reach_txpool_threshold<P, Block, ExPool, Balance>(
	parachain: &P,
	transaction_pool: Arc<ExPool>,
	height: RelayBlockNumber,
	snap_txs: Vec<H256>,
	core_price: Balance,
	relay_decimal: u64,
	parachain_decimal: u64,
) -> Option<(bool, OrderType)>
where
	Block: BlockT,
	P: ProvideRuntimeApi<Block> + UsageProvider<Block>,
	Balance: Codec + MaybeDisplay + 'static + Debug + AtLeast32BitUnsigned + Copy + From<u128>,
	P::Api: TransactionPaymentApi<Block, Balance>
		+ OrderRuntimeApi<Block, Balance>
		+ OnRelayChainApi<Block>,
	ExPool: MaintainedTransactionPool<Block = Block, Hash = <Block as BlockT>::Hash> + 'static,
{
	let mut pending_iterator = transaction_pool.ready();
	let mut is_place_order = false;
	let mut all_gas_value = Balance::from(0u32);
	let block_hash = parachain.usage_info().chain.best_hash;
	let mut back_txs: Vec<H256> = vec![];

	loop {
		let pending_tx =
			if let Some(pending_tx) = pending_iterator.next() { pending_tx } else { break };
		let pending_tx_data = pending_tx.data().clone();
		let utx_length = pending_tx_data.encode().len() as u32;
		let query_fee = parachain
			.runtime_api()
			.query_fee_details(block_hash, pending_tx_data, utx_length)
			.ok()?;
		all_gas_value = query_fee.final_fee().add(all_gas_value);
		if transaction_pool.status().ready != 0 {
			let new_core_price = if parachain_decimal >= relay_decimal {
				core_price.saturating_mul(
					(10 as u128).pow((parachain_decimal - relay_decimal) as u32).into(),
				)
			} else {
				core_price.checked_div(
					&(10 as u128).pow((parachain_decimal - relay_decimal) as u32).into(),
				)?
			};
			// Converted to a precision of 18
			is_place_order = parachain
				.runtime_api()
				.reach_txpool_threshold(block_hash, all_gas_value, new_core_price)
				.ok()?;
		}
		log::info!(
			"tx_fee:{:?}, all_fee:{:?}, core_price:{:?}, can_order:{:?}, status:{:?}",
			query_fee.final_fee(),
			all_gas_value,
			core_price,
			is_place_order,
			transaction_pool.status()
		);
		back_txs.push(H256::from_slice(pending_tx.hash().as_ref()));
	}
	let mut order_type = OrderType::Normal;
	if !is_place_order {
		//check is need force bid coretime
		let force_bid = parachain.runtime_api().on_relaychain(block_hash, height).ok()?;
		if all_gas_value.cmp(&Balance::from(0u32)) == Ordering::Greater && force_bid {
			is_place_order = true;
			order_type = OrderType::Force;
		}
	}
	if is_place_order {
		if back_txs == snap_txs {
			is_place_order = false;
		}
	}
	log::info!("back_txs:{:?}", back_txs);
	log::info!("snap_txs:{:?}", snap_txs);
	Some((is_place_order, order_type))
}

/// Whether the xcm transaction event of the relay chain is received.
async fn relay_chain_xcm_event(
	relay_chain_interface: impl RelayChainInterface + Clone,
	para_id: ParaId,
	relay_parent: H256,
) -> Option<(bool, OrderType)> {
	let downward_messages =
		relay_chain_interface.retrieve_dmq_contents(para_id, relay_parent).await.ok()?;
	let horizontal_messages = relay_chain_interface
		.retrieve_all_inbound_hrmp_channel_contents(para_id, relay_parent)
		.await
		.ok()?;
	let can_order = downward_messages.len() > 0 || horizontal_messages.len() > 0;
	return Some((can_order, OrderType::XCMEvent));
}

/// Get the transactions in the ready queue in the mem pool
async fn get_txs<Block, ExPool>(transaction_pool: Arc<ExPool>) -> Vec<H256>
where
	Block: BlockT,
	ExPool: MaintainedTransactionPool<Block = Block, Hash = <Block as BlockT>::Hash> + 'static,
{
	let mut pending_iterator = transaction_pool.ready();
	let mut back_txs: Vec<H256> = vec![];
	loop {
		let pending_tx =
			if let Some(pending_tx) = pending_iterator.next() { pending_tx } else { break };
		back_txs.push(H256::from_slice(pending_tx.hash().as_ref()));
	}
	return back_txs;
}

/// The main processing logic of purchasing core.
async fn handle_relaychain_stream<P, Block, ExPool, Balance>(
	validation_data: PersistedValidationData,
	height: RelayBlockNumber,
	parachain: &P,
	keystore: KeystorePtr,
	relay_chain: impl RelayChainInterface + Clone,
	p_hash: H256,
	para_id: ParaId,
	order_record: Arc<Mutex<OrderRecord>>,
	transaction_pool: Arc<ExPool>,
	url: String,
	relay_decimal: u64,
	parachain_decimal: u64,
) -> Result<(), Box<dyn Error>>
where
	Block: BlockT,
	P: ProvideRuntimeApi<Block> + UsageProvider<Block>,
	Balance: Codec
		+ MaybeDisplay
		+ 'static
		+ Debug
		+ Into<u128>
		+ AtLeast32BitUnsigned
		+ Copy
		+ From<u128>,
	P::Api: AuraApi<Block, AuthorityId>
		+ OrderRuntimeApi<Block, Balance>
		+ TransactionPaymentApi<Block, Balance>
		+ OnRelayChainApi<Block>,
	ExPool: MaintainedTransactionPool<Block = Block, Hash = <Block as BlockT>::Hash> + 'static,
{
	let is_parathread = is_parathread(&relay_chain, p_hash, para_id).await?;

	if !is_parathread {
		//parachain mode, clear all data.
		let mut order_record_local = order_record.lock().await;
		order_record_local.reset();
		return Ok(());
	} else {
		//parathread
		let p_on_demand = start_on_demand(relay_chain.clone(), p_hash, para_id).await;
		if let Some(on_demand) = p_on_demand {
			if !on_demand {
				// bulk mode
				return Ok(());
			}
		}
	}
	let head = validation_data.clone().parent_head.0;
	let parachain_head = match <<Block as BlockT>::Header>::decode(&mut &head[..]) {
		Ok(header) => header,
		Err(err) => return Err(format!("get parachain head error:{:?}", err).into()),
	};
	// parachain hash
	let hash = parachain_head.hash();
	let authorities = parachain.runtime_api().authorities(hash).map_err(Box::new)?;
	let slot_width = parachain.runtime_api().slot_width(hash)?;
	let auth_len = authorities.len() as u32;
	// The larger the slot width, the longer the rotation time.
	let idx = (height >> slot_width) % auth_len;
	// Randomly select a collator to place an order.
	let collator_public =
		mc_coretime_common::order_slot::<AuthorityPair>(idx, &authorities, &keystore).await;
	let base = 2 as u32;
	// Minimum interval for placing an order,calculated in one relaychain block time.
	let slot_block = base.pow(slot_width);
	if height % slot_block == 0 {
		let mut order_record_local = order_record.lock().await;
		order_record_local.reset();
	}
	let order_period = height & (slot_block - 1) < slot_block / 2;
	log::info!(
		"relaychain height:{:?},order period:{:?}, can place order:{:?}",
		height,
		order_period,
		collator_public.clone().is_some()
	);

	// Check whether the conditions for placing an order are met, and if so, place the order
	// get on demand core price
	let max_amount = parachain.runtime_api().order_max_amount(hash)?;
	let p_spot_price = get_spot_price::<Balance>(relay_chain.clone(), p_hash).await;
	let (spot_price, place_price) = if let Some(spot_price) = p_spot_price {
		if spot_price > max_amount {
			(spot_price, spot_price)
		} else {
			(spot_price, max_amount)
		}
	} else {
		(max_amount, max_amount)
	};
	// Check whether the gas of the transaction pool has reached the spot price threshold.
	let mut order_record_local = order_record.lock().await;

	let reached = reach_txpool_threshold(
		parachain,
		transaction_pool.clone(),
		height,
		order_record_local.txs.clone(),
		spot_price,
		relay_decimal,
		parachain_decimal,
	)
	.await;
	let mut can_order = false;
	let mut order_type = OrderType::Normal;
	if let Some((reach, o_t)) = reached {
		if reach {
			order_type = o_t;
			can_order = true;
		} else {
			let trig_xcm_event = relay_chain_xcm_event(relay_chain.clone(), para_id, p_hash).await;
			if let Some((trig_flag, o_t)) = trig_xcm_event {
				can_order = trig_flag;
				order_type = o_t;
			}
		}
	}
	if let Some(author) = collator_public {
		//your turn
		if can_order {
			if order_period {
				if order_record_local.order_status == OrderStatus::Init {
					log::info!("============place order, type:{:?}", order_type);
					order_record_local.relay_parent = p_hash;
					order_record_local.relay_height = height;
					order_record_local.author_pub = Some(author);
					order_record_local.txs = get_txs(transaction_pool).await;
					let order_result =
						try_place_order::<Balance>(keystore, para_id, url, place_price).await;
					order_record_local.order_status = OrderStatus::Order;
					if order_result.is_ok() {
						log::info!("===========place order successfully",);
					} else {
						log::info!("===========place order error:{:?}", order_result);
					}
				}
			}
		}
	}
	Ok(())
}

async fn new_best_heads(
	relay_chain: impl RelayChainInterface + Clone,
	para_id: ParaId,
) -> RelayChainResult<impl Stream<Item = (u32, PersistedValidationData, H256)>> {
	let new_best_notification_stream =
		relay_chain.new_best_notification_stream().await?.filter_map(move |n| {
			let relay_chain = relay_chain.clone();
			async move {
				let relay_head: PersistedValidationData = relay_chain
					.persisted_validation_data(n.hash(), para_id, OccupiedCoreAssumption::TimedOut)
					.await
					.map(|s| s.map(|s| s))
					.ok()
					.flatten()?;
				Some((n.number, relay_head, n.hash()))
			}
		});

	Ok(new_best_notification_stream)
}
async fn relay_chain_notification<P, R, Block, ExPool, Balance>(
	para_id: ParaId,
	parachain: Arc<P>,
	relay_chain: R,
	keystore: KeystorePtr,
	order_record: Arc<Mutex<OrderRecord>>,
	transaction_pool: Arc<ExPool>,
	url: String,
	parachain_decimal: u64,
) where
	R: RelayChainInterface + Clone,
	Block: BlockT,
	Balance: Codec
		+ MaybeDisplay
		+ 'static
		+ Debug
		+ Into<u128>
		+ AtLeast32BitUnsigned
		+ Copy
		+ From<u128>,
	P: ProvideRuntimeApi<Block> + UsageProvider<Block>,
	P::Api: AuraApi<Block, AuthorityId>
		+ OrderRuntimeApi<Block, Balance>
		+ TransactionPaymentApi<Block, Balance>
		+ OnRelayChainApi<Block>,
	ExPool: MaintainedTransactionPool<Block = Block, Hash = <Block as BlockT>::Hash> + 'static,
{
	let rpc_client = RpcClient::from_url(url.clone()).await.expect("rpc connect failed");
	let rpc = LegacyRpcMethods::<PolkadotConfig>::new(rpc_client.clone());
	let properties = rpc.system_properties().await.expect("can't get relaychain properties");
	let relay_decimal = properties
		.get("tokenDecimals")
		.and_then(|v| v.as_u64())
		.expect("can't get relaychain token decimal");

	let new_best_heads = match new_best_heads(relay_chain.clone(), para_id).await {
		Ok(best_heads_stream) => best_heads_stream.fuse(),
		Err(_err) => {
			return;
		},
	};
	pin_mut!(new_best_heads);
	loop {
		select! {
			h = new_best_heads.next() => {
				match h {
					Some((height, head, hash)) => {
						let _ = handle_relaychain_stream(head,height, &*parachain,keystore.clone(), relay_chain.clone(), hash, para_id, order_record.clone(),transaction_pool.clone(), url.clone(), relay_decimal, parachain_decimal).await;
					},
					None => {
						return;
					}
				}
			},
		}
	}
}

pub async fn ondemand_event_task(
	para_id: ParaId,
	rpc_url: String,
	order_record: Arc<Mutex<OrderRecord>>,
) -> Result<(), Box<dyn Error>> {
	// Get the final block of the relaychain through subxt.

	let api = OnlineClient::<PolkadotConfig>::from_url(rpc_url.clone()).await?;

	let mut blocks_sub = api.blocks().subscribe_best().await?;

	// For each block, print a bunch of information about it:
	while let Some(block) = blocks_sub.next().await {
		let block = block?;

		let events = block.events().await?;
		for event in events.iter() {
			let event = event?;
			// Query Broker Assigned Event
			let ev_order_placed = event.as_event::<metadata::OnDemandOrderPlacedV0>();
			if let Ok(order_placed_event) = ev_order_placed {
				if let Some(ev) = order_placed_event {
					log::info!(
						"=====================Find OnDemandOrderPlaced event:{:?},{:?}================",
						ev.para_id,
						ev.spot_price,
					);
					let exp_id: u32 = para_id.into();
					if ev.para_id.0 == exp_id {
						// The orderer gets it from the slot by default.
						let mut order_record_local = order_record.lock().await;
						order_record_local.price = ev.spot_price;
						order_record_local.order_status = OrderStatus::Execute;
					}
				}
			}
		}
	}
	Ok(())
}

async fn event_notification(para_id: ParaId, url: String, order_record: Arc<Mutex<OrderRecord>>) {
	loop {
		let _ = ondemand_event_task(para_id, url.clone(), order_record.clone()).await;
	}
}

pub async fn run_on_demand_task<P, R, Block, ExPool, Balance>(
	para_id: ParaId,
	parachain: Arc<P>,
	relay_chain: R,
	keystore: KeystorePtr,
	order_record: Arc<Mutex<OrderRecord>>,
	transaction_pool: Arc<ExPool>,
	url: String,
	parachain_decimal: u64,
) where
	R: RelayChainInterface + Clone,
	Block: BlockT,
	P: ProvideRuntimeApi<Block> + UsageProvider<Block>,
	Balance: Codec
		+ MaybeDisplay
		+ 'static
		+ Debug
		+ Into<u128>
		+ AtLeast32BitUnsigned
		+ Copy
		+ From<u128>,
	P::Api: AuraApi<Block, AuthorityId>
		+ OrderRuntimeApi<Block, Balance>
		+ TransactionPaymentApi<Block, Balance>
		+ OnRelayChainApi<Block>,
	ExPool: MaintainedTransactionPool<Block = Block, Hash = <Block as BlockT>::Hash> + 'static,
{
	let relay_chain_notification = relay_chain_notification(
		para_id,
		parachain.clone(),
		relay_chain.clone(),
		keystore,
		order_record.clone(),
		transaction_pool,
		url.clone(),
		parachain_decimal,
	);
	let event_notification = event_notification(para_id, url, order_record);
	select! {
		_ = relay_chain_notification.fuse() => {},
		_ = event_notification.fuse() => {},
	}
}

pub fn spawn_on_demand_order<T, R, ExPool, Block, Balance>(
	parachain: Arc<T>,
	para_id: ParaId,
	relay_chain: R,
	transaction_pool: Arc<ExPool>,
	task_manager: &TaskManager,
	keystore: KeystorePtr,
	order_record: Arc<Mutex<OrderRecord>>,
	relay_rpc: Option<SocketAddr>,
	parachain_decimal: u64,
) -> sc_service::error::Result<()>
where
	Block: BlockT,
	R: RelayChainInterface + Clone + 'static,
	Balance: Codec
		+ MaybeDisplay
		+ 'static
		+ Debug
		+ Send
		+ Into<u128>
		+ AtLeast32BitUnsigned
		+ Copy
		+ From<u128>,
	T: Send + Sync + 'static + ProvideRuntimeApi<Block> + UsageProvider<Block>,
	ExPool: MaintainedTransactionPool<Block = Block, Hash = <Block as BlockT>::Hash> + 'static,
	T::Api: AuraApi<Block, AuthorityId>
		+ OrderRuntimeApi<Block, Balance>
		+ TransactionPaymentApi<Block, Balance>
		+ OnRelayChainApi<Block>,
{
	let mut url = String::from("ws://");
	url.push_str(&relay_rpc.expect("Should set rpc address for submit order extrinic").to_string());

	let on_demand_order_task = run_on_demand_task(
		para_id,
		parachain.clone(),
		relay_chain.clone(),
		keystore,
		order_record,
		transaction_pool.clone(),
		url,
		parachain_decimal,
	);
	task_manager.spawn_essential_handle().spawn_blocking(
		"on demand order task",
		"coretime",
		on_demand_order_task,
	);
	Ok(())
}
