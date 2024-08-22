pub(crate) use crate as pallet_liquidation;
pub(crate) use crate::Event as LiquidationEvent;
use codec::Encode;
use frame_support::traits::{Everything, Nothing};
use frame_support::{
	parameter_types,
	traits::{ConstU32, ConstU64},
	weights::{
		constants::ExtrinsicBaseWeight, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
};
use frame_system as system;
use frame_system::EnsureRoot;
use smallvec::smallvec;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage, Perbill,
};

use crate::OrderGasCost;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_std::{cell::RefCell, collections::btree_map::BTreeMap};
use xcm::latest::{Assets, Location, SendError, SendResult, SendXcm, Xcm, XcmHash};
use xcm::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, FixedRateOfFungible, FixedWeightBounds, IsConcrete,
	SignedToAccountId32, TakeWeightCredit,
};
use xcm_executor::XcmExecutor;

type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

pub const UNIT: Balance = 1_000_000_000_000_000_000;
const MILLIUNIT: Balance = 1_000_000_000;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		// OrderPallet: pallet_order::{Pallet, Storage, Event<T>},
		Pot: pallet_pot::{Pallet, Call, Storage, Event<T>},
		Utility: pallet_utility::{Pallet, Call, Storage, Event},
		Liquidation: pallet_liquidation::{Pallet, Storage, Event<T>},
		XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config<T>},
	}
);

//ALICE
const COLLATOR_BYTES: [u8; 32] = [
	212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133,
	76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
];
const SYSTEM_ACCOUNT_BYTES: [u8; 32] = [
	54, 99, 32, 239, 79, 115, 118, 121, 15, 239, 57, 41, 2, 255, 91, 189, 21, 193, 175, 83, 111,
	196, 75, 126, 82, 14, 205, 184, 6, 168, 148, 234,
];

const COLLATOR: AccountId32 = AccountId32::new(COLLATOR_BYTES);
const SYSTEM_ACCOUNT: AccountId32 = AccountId32::new(SYSTEM_ACCOUNT_BYTES);

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Nonce = u64;
	type Block = Block;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<5>;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeTask = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = EXISTENTIAL_DEPOSIT;
	pub const SystemAccountName: &'static str = "system";
	pub const TreasuryAccountName: &'static str = "treasury";
	pub const OperationAccountName: &'static str = "maintenance";
}

impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeFreezeReason = ();
}

impl pallet_utility::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

// parameter_types! {
// 	pub const SlotWidth: u32 = 2;
// 	pub const OrderMaxAmount:Balance = 200000000;
// 	pub const TxPoolThreshold:Balance = 3000000000;
// }

// impl pallet_order::Config for Test {
// 	type RuntimeEvent = RuntimeEvent;
// 	type AuthorityId = AuraId;
// 	type Currency = Balances;
// 	type UpdateOrigin = frame_system::EnsureRoot<Self::AccountId>;
// 	type OrderMaxAmount = OrderMaxAmount;
// 	type SlotWidth = SlotWidth;
// 	type TxPoolThreshold = TxPoolThreshold;
// 	type WeightInfo = ();
// }

use pallet_pot::PotNameBtreemap;
use pallet_xcm::TestWeightInfo;

pub type AccountId = AccountId32;
parameter_types! {
	pub const PotNames: [&'static str;3] = ["system", "treasury", "maintenance"];
	pub Pots: BTreeMap<String, AccountId> = pallet_pot
											::HashedPotNameBtreemap
											::<Test, pallet_pot::HashedPotNameMapping<BlakeTwo256>>
											::pots_btreemap(&(PotNames::get()));
}
impl pallet_pot::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PotNameMapping = pallet_pot::HashedPotNameMapping<BlakeTwo256>;
	type Currency = Balances;
	type Pots = Pots;
}

thread_local! {
	pub static SENT_XCM: RefCell<Vec<(Location, Xcm<()>)>> = RefCell::new(Vec::new());
}

pub(crate) fn fake_message_hash<T>(message: &Xcm<T>) -> XcmHash {
	message.using_encoded(sp_io::hashing::blake2_256)
}

pub struct TestSendXcm;
impl SendXcm for TestSendXcm {
	type Ticket = (Location, Xcm<()>);
	fn validate(
		dest: &mut Option<Location>,
		msg: &mut Option<Xcm<()>>,
	) -> SendResult<(Location, Xcm<()>)> {
		let pair = (dest.take().unwrap(), msg.take().unwrap());
		Ok((pair, Assets::new()))
	}
	fn deliver(pair: (Location, Xcm<()>)) -> Result<XcmHash, SendError> {
		let hash = fake_message_hash(&pair.1);
		SENT_XCM.with(|q| q.borrow_mut().push(pair));
		Ok(hash)
	}
}

parameter_types! {
	pub const BaseXcmWeight: Weight = Weight::from_parts(1_000, 1_000);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub XcmFeesTargetAccount: AccountId = AccountId::new([167u8; 32]);
	pub UniversalLocation: InteriorLocation = Here;
	pub const AnyNetwork: Option<NetworkId> = None;
	pub const RelayLocation: Location = Here.into_location();
	pub CurrencyPerSecondPerByte: (AssetId, u128, u128) = (AssetId(RelayLocation::get()), 1, 1);
}

pub struct XcmConfig;
pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowKnownQueryResponses<XcmPallet>,
	AllowSubscriptionsFrom<Everything>,
);

impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = ();
	type OriginConverter = ();
	type IsReserve = ();
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type Trader = FixedRateOfFungible<CurrencyPerSecondPerByte, ()>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = ();
	//type FeeManager = XcmFeeManagerFromComponents<
	//	EverythingBut<XcmFeesNotWaivedLocations>,
	//	XcmFeeToAccount<Self::AssetTransactor, AccountId, XcmFeesTargetAccount>,
	//>;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = ();
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = ();
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, AnyNetwork>;

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = ();
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type AdminOrigin = EnsureRoot<AccountId>;
	type TrustedLockers = ();
	type SovereignAccountOf = AccountId32Aliases<(), AccountId32>;
	type Currency = Balances;
	type CurrencyMatcher = IsConcrete<RelayLocation>;
	type MaxLockers = frame_support::traits::ConstU32<8>;
	type MaxRemoteLockConsumers = frame_support::traits::ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = TestWeightInfo;
}

pub type XcmRouter = TestSendXcm;

impl pallet_liquidation::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type XcmSender = XcmRouter;
	type WeightToFee = WeightToFee;
	type OrderGasCost = MockOrderGasCostHandler;
	type ExistentialDeposit = ExistentialDeposit;
	type SystemAccountName = SystemAccountName;
	type TreasuryAccountName = TreasuryAccountName;
	type OperationAccountName = OperationAccountName;
}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let p = MILLIUNIT / 10;
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

pub struct MockOrderGasCostHandler;
impl<T: pallet_liquidation::Config> OrderGasCost<T> for MockOrderGasCostHandler
where
	T: crate::Config,
	T::AccountId: From<[u8; 32]>,
{
	fn gas_cost(
		_block_number: BlockNumberFor<T>,
	) -> Result<Option<(T::AccountId, Balance)>, sp_runtime::DispatchError> {
		let account = T::AccountId::try_from(COLLATOR_BYTES).unwrap();
		Ok(Some((account, 10000000 as u128)))
	}
}

pub struct ExtBuilder {
	existential_deposit: u128,
	system_ratio: u32,
	treasury_ratio: u32,
	operation_ratios: Vec<(AccountId, u32)>,
	collator_ratio: u32,
	min_liquidation_threshold: Balance,
	profit_distribution_cycle: u64,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 1,
			system_ratio: 20_000_0000,
			treasury_ratio: 33_000_0000,
			operation_ratios: vec![(AccountId32::new([1u8; 32]), 25_000_000)],
			collator_ratio: 22_000_0000,
			min_liquidation_threshold: MILLIUNIT * 20,
			profit_distribution_cycle: 10,
		}
	}
}

impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u128) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}

	pub fn system_ratio(mut self, ratio: u32) -> Self {
		self.system_ratio = ratio;
		self
	}

	pub fn treasury_ratio(mut self, ratio: u32) -> Self {
		self.treasury_ratio = ratio;
		self
	}

	pub fn operation_ratio(mut self, ratio: Vec<(AccountId, u32)>) -> Self {
		self.operation_ratios = ratio;
		self
	}

	pub fn collator_ratio(mut self, ratio: u32) -> Self {
		self.collator_ratio = ratio;
		self
	}

	pub fn min_liquidation_threshold(mut self, threshold: Balance) -> Self {
		self.min_liquidation_threshold = threshold;
		self
	}

	pub fn profit_distribution_cycle(mut self, cycle: u64) -> Self {
		self.profit_distribution_cycle = cycle;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		let balances_config = pallet_balances::GenesisConfig::<Test> {
			balances: vec![(COLLATOR, UNIT), (SYSTEM_ACCOUNT, UNIT)],
		};
		balances_config.assimilate_storage(&mut storage).unwrap();

		let liquidation_config = pallet_liquidation::GenesisConfig::<Test> {
			admin_key: Some(SYSTEM_ACCOUNT),
			system_ratio: self.system_ratio,
			treasury_ratio: self.treasury_ratio,
			operation_ratios: self.operation_ratios.clone(),
			collator_ratio: self.collator_ratio,
			min_liquidation_threshold: self.min_liquidation_threshold,
			profit_distribution_cycle: self.profit_distribution_cycle,
		};
		liquidation_config.assimilate_storage(&mut storage).unwrap();
		let mut ext = sp_io::TestExternalities::new(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn expected_event(event: &LiquidationEvent<Test>) -> bool {
	matches!(
		event,
		LiquidationEvent::ProfitDistributed(_, _) | LiquidationEvent::CollatorsCompensated(_, _)
	)
}
