use cumulus_primitives_core::ParaId;
use parachain_magnet_runtime::{AccountId, AuraId, Signature, EXISTENTIAL_DEPOSIT};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::Ss58Codec, sr25519, Pair, Public, H160, H256, U256};
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::AccountId32;
use sp_std::marker::PhantomData;
use std::{collections::BTreeMap, str::FromStr};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
	sc_service::GenericChainSpec<parachain_magnet_runtime::RuntimeGenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> parachain_magnet_runtime::SessionKeys {
	parachain_magnet_runtime::SessionKeys { aura: keys }
}

pub fn development_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	ChainSpec::builder(
		parachain_magnet_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(),
			// You MUST set this to the correct network!
			para_id: 2000,
		},
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_collator_keys_from_seed("Alice"),
			),
			(
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_collator_keys_from_seed("Bob"),
			),
		],
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		2000.into(),
	))
	.build()
}

pub fn local_testnet_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());

	#[allow(deprecated)]
	ChainSpec::builder(
		parachain_magnet_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(),
			// You MUST set this to the correct network!
			para_id: 2000,
		},
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_collator_keys_from_seed("Alice"),
			),
			(
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_collator_keys_from_seed("Bob"),
			),
		],
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		2000.into(),
	))
	.with_protocol_id("magnet-local")
	.with_properties(properties)
	.build()
}

fn get_account_id_from_address(address: &str) -> AccountId32 {
	AccountId32::from_ss58check(address).expect("Invalid address")
}
fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	root: AccountId,
	id: ParaId,
) -> serde_json::Value {
	let alice = get_from_seed::<sr25519::Public>("Alice");
	let bob = get_from_seed::<sr25519::Public>("Bob");

	let op_account1 =
		get_account_id_from_address("5GP7etLvS2VLLfUar7Q2TkQkaxHweYnDvrhh3s5hhf8eorPW");
	let op_account2 =
		get_account_id_from_address("5CFuj7WxZAyinLxoqAJ8NH4yEEVXUUSHi9LRhodC3HyzHvN4");

	let evm_accounts = {
		let mut map = BTreeMap::new();
		map.insert(
			// H160 address of Alice dev account
			// Derived from SS58 (42 prefix) address
			// SS58: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
			// hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
			// Using the full hex key, truncating to the first 20 bytes (the first 40 hex
			// chars)
			H160::from_str("d43593c715fdd31c61141abd04a99fd6822c8558")
				.expect("internal H160 is valid; qed"),
			fp_evm::GenesisAccount {
				balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
					.expect("internal U256 is valid; qed"),
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default(),
			},
		);
		map.insert(
			// H160 address of CI test runner account
			H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b")
				.expect("internal H160 is valid; qed"),
			fp_evm::GenesisAccount {
				balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
					.expect("internal U256 is valid; qed"),
				code: Default::default(),
				nonce: Default::default(),
				storage: Default::default(),
			},
		);
		map.insert(
			// H160 address for benchmark usage
			H160::from_str("1000000000000000000000000000000000000001")
				.expect("internal H160 is valid; qed"),
			fp_evm::GenesisAccount {
				nonce: U256::from(1),
				balance: U256::from(1_000_000_000_000_000_000_000_000u128),
				storage: Default::default(),
				code: vec![0x00],
			},
		);
		map
	};

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 1u128 << 81)).collect::<Vec<_>>(),
		},
		"assets": {
			"assets": vec![
				(1, alice, true, 1_000_000_0000_0000_0000u128),
				(2, bob, true, 2_000_000_0000_0000_0000u128),
			],
			// Genesis metadata: Vec<(id, name, symbol, decimals)>
			"metadata": vec![
				(1, b"asset-1".to_vec(), b"ALT1".to_vec(), 18),
				(2, b"asset-2".to_vec(), b"ALT2".to_vec(), 18),
			],
			// Genesis accounts: Vec<(id, account_id, balance)>
			"accounts": vec![
				(1, alice, 500_000_000_0000_0000_0000u128),
				(2, bob, 500_000_000_0000_0000_0000u128),
			],
		},
		"assetsBridge": {
			"adminKey": Some(root.clone()),
		},
		"council": {
			"members": endowed_accounts
				.iter()
				.enumerate()
				.filter_map(|(idx, acc)| if idx % 2 == 0 { Some(acc.clone()) } else { None })
				.collect::<Vec<_>>(),
		},
		"parachainInfo": {
			"parachainId": id,
		},
		"collatorSelection": {
			"invulnerables": invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
			"candidacyBond": EXISTENTIAL_DEPOSIT * 16,
		},
		"session": {
			"keys": invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						template_session_keys(aura), // session keys
					)
				})
				.collect::<Vec<_>>(),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"sudo": { "key": Some(root.clone()) },

		// EVM compatibility
		"evmChainId": {
			"chainId": u64::from(u32::from(id)),
		},

		"evm": { "accounts": evm_accounts },
		//Move VM
		"moveModule": {
			"changeDefaultMoveStdlibBundleTo": Option::<Vec<u8>>::None,
			"changeDefaultSubstrateStdlibBundleTo": Option::<Vec<u8>>::None,
		},
		"bulkPallet":{
			"rpcUrl": b"ws://127.0.0.1:8855".to_vec(),
			"genesisHash": H256::from_str("0x016f9d0bc355e718ce950727cd423d4915f34ded0a94f466242446b8865e061f").expect("genesis hash error.")
		},
		"orderPallet": {
			"slotWidth": 3,
			"priceLimit": 200000000,
			"gasThreshold": 10,
		},
		"liquidation":{
			"adminKey": Some(root.clone()),
			"systemRatio": 20_000_0000,
			"treasuryRatio": 33_000_0000,
			"operationRatios": vec![
				(op_account1.clone(), 15_000_0000),
				(op_account2.clone(), 10_000_0000)
			],
			"collatorRatio": 22_000_0000,
			"minLiquidationThreshold": 20_000_000_000_000_000u128,
			"profitDistributionCycle": 10,
		}
	})
}
