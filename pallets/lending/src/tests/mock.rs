use crate as pallet_template;
use crate::{AssetBalanceOf, AssetIdOf, BalanceOf};
pub type Fungibles = <Test as crate::Config>::Fungibles;
use frame_support::{
	assert_ok, derive_impl, parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, ConstU64, OnFinalize, OnInitialize,
	},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use once_cell::sync::OnceCell;
use sp_core::H256;
use sp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	BuildStorage, FixedU128,
};
use std::{cell::RefCell, collections::HashSet};

// Define a static OnceCell to ensure the logger is initialized only once
static INIT_LOGGER: OnceCell<()> = OnceCell::new();

/// Initializes the logger for tests.
/// This should be called at the beginning of your tests.
fn initialize_logger() {
	INIT_LOGGER.get_or_init(|| {
		let _ = env_logger::builder().is_test(true).try_init();
	});
}

pub type AssetId = u32;
pub type AccountId = u64;
type Block = frame_system::mocking::MockBlock<Test>;

type Balance = u128;
pub const ADMIN: AccountId = 1;
pub const ALICE: AccountId = 2;
pub const BOB: AccountId = 3;
pub const USDT: AssetId = 1u32;
pub const DOT: AssetId = 2u32;
pub const KSM: AssetId = 3u32;

pub const LENDING_POOL_TOKEN: AssetId = 99999u32;
pub type Rate = FixedU128;
const BLOCK_TIME_MS: u64 = 6_000; // 6 seconds per block in milliseconds

thread_local! {
	pub static ENDOWED_BALANCES: RefCell<Vec<(AssetId, AccountId, Balance)>> = RefCell::new(Vec::new());
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		TemplateModule: pallet_template,
		Timestamp: pallet_timestamp,
	}
);
#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxLocks = ConstU32<10>;
	type MaxReserves = ();
	type MaxHolds = ConstU32<10>;
	type MaxFreezes = ConstU32<10>;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<Self::AccountId>>;
	type ForceOrigin = EnsureRoot<Self::AccountId>;
	type AssetDeposit = ConstU128<100>;
	type AssetAccountDeposit = ConstU128<1>;
	type MetadataDepositBase = ConstU128<10>;
	type MetadataDepositPerByte = ConstU128<1>;
	type ApprovalDeposit = ConstU128<1>;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const KylixPalletId: PalletId = PalletId(*b"kylixpdl");
}

impl pallet_template::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Time = Timestamp;
	#[doc = r" Type to access the Balances Pallet."]
	type NativeBalance = Balances;
	type WeightInfo = ();
	#[doc = r" Type to access the Assets Pallet."]
	type Fungibles = Assets;
	type PalletId = KylixPalletId;
	//	#[doc = r" The origin which can add or remove LendingPools and update LendingPools (interest
	// rate model, kink, activate, deactivate)."] 	type ManagerOrigin = ();
}

pub struct ExtBuilder {
	endowed_balances: Vec<(AssetId, AccountId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		ENDOWED_BALANCES.with(|v| {
			v.borrow_mut().clear();
		});
		Self { endowed_balances: vec![] }
	}
}

impl ExtBuilder {
	pub fn with_endowed_balances(mut self, balances: Vec<(AssetId, AccountId, Balance)>) -> Self {
		self.endowed_balances = balances;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		initialize_logger();
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		let mut unique_assets = HashSet::new();
		let mut assets = vec![];
		//Genesis metadata: id, name, symbol, decimals
		let mut metadata = vec![];
		for (asset_id, _, _) in self.endowed_balances.clone().into_iter() {
			if unique_assets.insert(asset_id) {
				// Only push the asset if it wasn't already in the set
				assets.push((asset_id, ADMIN, true, 1));
				metadata.push((
					asset_id,
					format!("{}Name", asset_id).into_bytes(),
					format!("{}Symbol", asset_id).into_bytes(),
					18,
				));
			}
		}

		pallet_assets::GenesisConfig::<Test> { assets, metadata, accounts: self.endowed_balances }
			.assimilate_storage(&mut t)
			.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub fn setup_active_pool(asset: AssetIdOf<Test>, initial_balance: BalanceOf<Test>) {
	assert_ok!(TemplateModule::create_lending_pool(
		RuntimeOrigin::signed(ALICE),
		LENDING_POOL_TOKEN,
		asset,
		initial_balance
	));
	assert_ok!(TemplateModule::activate_lending_pool(RuntimeOrigin::signed(ALICE), asset));
}

pub fn get_pallet_balance(asset: AssetIdOf<Test>) -> AssetBalanceOf<Test> {
	let pallet_account: AccountId = KylixPalletId::get().into_account_truncating();
	return Fungibles::balance(asset, pallet_account);
}

pub fn run_to_block(n: u64) {
    let current_block = System::block_number();
    if n > current_block {
        TemplateModule::on_finalize(current_block);
        System::set_block_number(n);
        let time_difference = (n - current_block) * BLOCK_TIME_MS;
        Timestamp::set_timestamp(Timestamp::get() + time_difference);
        TemplateModule::on_initialize(System::block_number());
    }
}
