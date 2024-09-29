#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Decode, Encode};
use frame_support::traits::AsEnsureOriginWithArg;
use frame_system::{EnsureRoot, EnsureSigned};
use lending::{fungibles::metadata::Inspect, FixedU128};
use pallet_grandpa::AuthorityId as GrandpaId;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_api::{decl_runtime_apis, impl_runtime_apis};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU64, MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness,
		StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	StorageValue,
};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{traits::Zero, Perbill, Permill, SaturatedConversion};

/// Import the lending pallet.
pub use lending;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Total value of all deposits in USDT for a given account.
pub type TotalDeposit = Balance;

/// Total value of all borrow assets in USDT for a given account.
pub type TotalBorrow = Balance;

/// Total value of all collateral in USDT for a given account.
pub type TotalCollateral = Balance;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("kylix-node"),
	impl_name: create_runtime_str!("kylix-node"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(
			Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
			NORMAL_DISPATCH_RATIO,
		);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type MaxHolds = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

use frame_support::PalletId;

parameter_types! {
	pub const LendingPalletId: PalletId = PalletId(*b"kylix_id");
}

/// Configure the lending in pallets/lending.
impl lending::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = lending::weights::SubstrateWeight<Runtime>;
	type NativeBalance = Balances;
	type Fungibles = Assets;
	type PalletId = LendingPalletId;
	type Time = Timestamp;
}

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<1>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct LendingPoolInfo {
	pub id: u32,
	pub asset_id: u32,
	pub asset: Vec<u8>,
	pub asset_decimals: u32,
	pub asset_symbol: Vec<u8>,
	pub collateral_q: u64,
	pub utilization: FixedU64,
	pub borrow_apy: FixedU128,
	pub supply_apy: FixedU128,
	pub is_activated: bool,
	pub balance: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct AssetInfo {
    asset_id: u32,
    asset_symbol: Vec<u8>,
    asset_name: Vec<u8>,
    decimals: u8,
    asset_icon: Vec<u8>,
    balance: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct BorrowedAsset {
	#[serde(flatten)]
    asset_info: AssetInfo, 
    apy: FixedU128,
    borrowed: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct SuppliedAsset {
    #[serde(flatten)]
    asset_info: AssetInfo,
    apy: FixedU128,
    supplied: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct CollateralAsset {
    #[serde(flatten)]
    asset_info: AssetInfo,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct UserLTVInfo {
	pub current_ltv: FixedU128,
	pub sale_ltv: FixedU128,
	pub liquidation_ltv: FixedU128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct AggregatedTotals {
	pub total_supply: u128,
	pub total_borrow: u128,
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,
		Assets: pallet_assets,
		// Include the custom logic from the lending in the runtime.
		Lending: lending,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_sudo, Sudo]
		[lending, Lending]
	);
}

decl_runtime_apis! {
	pub trait LendingPoolApi {
		fn get_lending_pools() -> (Vec<LendingPoolInfo>, AggregatedTotals);
		fn get_user_ltv(account: AccountId) -> UserLTVInfo;
        fn get_asset_wise_supplies(account: AccountId) -> (Vec<SuppliedAsset>, TotalDeposit);
		fn get_asset_wise_borrows_collaterals(account: AccountId) -> (Vec<BorrowedAsset>, Vec<CollateralAsset>, TotalBorrow, TotalCollateral);
    }
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	impl crate::LendingPoolApi<Block> for Runtime {
		fn get_user_ltv(account: AccountId) -> UserLTVInfo {
			let (current_ltv, sale_ltv, liquidation_ltv) = Lending::compute_user_ltv(&account);
			UserLTVInfo {
				current_ltv,
				sale_ltv,
				liquidation_ltv,
			}
		}

		fn get_lending_pools() -> (Vec<LendingPoolInfo>, AggregatedTotals) {
			let mut total_supply: u128 = 0;
			let mut total_borrow: u128 = 0;

			// Collect all the lending pools and aggregate totals in a single iteration
			let pools: Vec<LendingPoolInfo> = lending::LendingPoolStorage::<Runtime>::iter()
				.map(|(_, pool)| {
					// Use the InspectMetadata trait to get the asset name and decimals
					let asset_name = pallet_assets::Pallet::<Runtime>::name(pool.lend_token_id);
					let asset_decimals = pallet_assets::Pallet::<Runtime>::decimals(pool.lend_token_id);
					let asset_symbol = pallet_assets::Pallet::<Runtime>::symbol(pool.lend_token_id);

					// Calculate the balance=reserve_balance+borrowed_balance
					let balance = pool.reserve_balance.saturating_add(pool.borrowed_balance).into();

					let equivalent_asset_supply_amount = lending::Pallet::<Runtime>::get_equivalent_asset_amount(
						pool.lend_token_id,
						1, //USDT
						pool.reserve_balance,
					).unwrap_or_default()
					;
					let equivalent_asset_borrow_amount = lending::Pallet::<Runtime>::get_equivalent_asset_amount(
						pool.lend_token_id,
						1, //USDT
						pool.borrowed_balance,
					).unwrap_or_default();

					// Accumulate totals
					total_supply = total_supply.saturating_add(equivalent_asset_supply_amount);
					total_borrow = total_borrow.saturating_add(equivalent_asset_borrow_amount);

					LendingPoolInfo {
						id: pool.id,
						asset_id: pool.lend_token_id,
						asset: asset_name,
						asset_decimals: asset_decimals as u32,
						asset_symbol: asset_symbol,
						collateral_q: pool.collateral_factor.deconstruct().into(),
						utilization: pool.utilisation_ratio().unwrap_or_default().into(),
						borrow_apy: pool.borrow_interest_rate().unwrap_or_default().into(),
						supply_apy: pool.supply_interest_rate().unwrap_or_default().into(),
						is_activated: pool.activated,
						balance,
					}
				})
				.collect();

			let aggregated_totals = AggregatedTotals {
				total_supply,
				total_borrow,
			};

			// Return the pools and aggregated totals
			(pools, aggregated_totals)
		}

		fn get_asset_wise_supplies(account: AccountId) -> (Vec<SuppliedAsset>, TotalDeposit) {
			// Initialize the total deposit to zero
			let mut total_supply: u128 = 0;
		
			// Iterate over all lending pools and gather asset supplies for the given account
			let supplied_assets: Vec<SuppliedAsset> = lending::LendingPoolStorage::<Runtime>::iter()
				.filter_map(|(_, mut pool)| {
					// Get the account's LP token balance for this pool
					let lp_balance = lending::Pallet::<Runtime>::get_asset_balance(&account, pool.id).ok()?;

					// Skip this pool if the balance is zero
					if lp_balance == 0 {
						return None;
					}
					
					// Update pool indexes
					if pool.update_indexes().is_err() {
						return None;
					}

					// Calculate the supplied amount value by supply_index `lp_balance * supply_index`
					let supplied_amount = pool.accrued_deposit(lp_balance).ok()?;

					let asset_balance = lending::Pallet::<Runtime>::get_asset_balance(&account, pool.lend_token_id).ok()?;

					// Retrieve metadata for the pool's asset
					let asset_name = pallet_assets::Pallet::<Runtime>::name(pool.lend_token_id);
					let asset_decimals = pallet_assets::Pallet::<Runtime>::decimals(pool.lend_token_id);
					let asset_symbol = pallet_assets::Pallet::<Runtime>::symbol(pool.lend_token_id);
					let asset_icon = "<url>/dot.svg".as_bytes().to_vec();  // temporarily mocked
					
					// Calculate the equivalent supplied amount
					let equivalent_supplied_amount = lending::Pallet::<Runtime>::get_equivalent_asset_amount(
						pool.lend_token_id,
						1, // USDT
						supplied_amount,
					).unwrap_or_default();
		
					// Calculate the current APY for this pool
					let apy = pool.supply_interest_rate().unwrap_or_default();
		
					// Accumulate total supply
					total_supply = total_supply.saturating_add(equivalent_supplied_amount);
		
					// Create a `SuppliedAsset` for the current pool
					Some(SuppliedAsset {
						asset_info: AssetInfo {
							asset_id: pool.lend_token_id,
							asset_name: asset_name,
							asset_symbol: asset_symbol,
							decimals: asset_decimals,
							asset_icon: asset_icon,
							balance: asset_balance,
						},
						apy: apy,
						supplied: supplied_amount,
					})
				})
				.collect();
		
			// Wrap up and return the assets and total deposits
			(supplied_assets, total_supply)
		}

		fn get_asset_wise_borrows_collaterals(account: AccountId) -> (Vec<BorrowedAsset>, Vec<CollateralAsset>, u128, u128) {
			// Initialize the total borrow and collateral amounts
			let mut total_borrow: u128 = 0;
			let mut total_collateral: u128 = 0;
		
			// Initialize vectors to store borrowed assets and collateral assets
			let mut borrowed_assets: Vec<BorrowedAsset> = Vec::new();
			let mut collateral_assets: Vec<CollateralAsset> = Vec::new();
		
			// Iterate over all borrows for the account
			for ((borrower, borrowed_asset, collateral_asset), loan) in lending::Borrows::<Runtime>::iter() {
				// Ensure we are processing only entries for the given account
				if borrower != account {
					continue;
				}
		
				// Get the lending pool for the borrowed asset
				let asset_pool = lending::AssetPool::<Runtime>::from(borrowed_asset);
				let mut pool = match lending::LendingPoolStorage::<Runtime>::get(&asset_pool) {
					Some(p) => p,
					None => continue, // Skip if no pool found
				};

				// Update pool indexes
				if pool.update_indexes().is_err() {
					continue;
				}

				// Compute the repayable amount (current borrowed balance with interest)
				let borrowed_amount = match pool.repayable_amount(&loan) {
					Ok(amount) => amount,
					Err(_) => continue,
				};

				// Handle Borrowed Assets
				let borrow_balance = match lending::Pallet::<Runtime>::get_asset_balance(&account, borrowed_asset){
					Ok(amount) => amount,
					Err(_) => continue,
				};
		
				// Retrieve asset metadata for the borrowed asset
				let borrow_asset_name = pallet_assets::Pallet::<Runtime>::name(borrowed_asset);
				let borrow_asset_symbol = pallet_assets::Pallet::<Runtime>::symbol(borrowed_asset);
				let borrow_asset_decimals = pallet_assets::Pallet::<Runtime>::decimals(borrowed_asset);
				let borrow_asset_icon = "<url>/dot.svg".as_bytes().to_vec(); // Mocked for now
	
				// Calculate equivalent borrowed amount in USDT (or any other base token)
				let equivalent_borrowed_amount = lending::Pallet::<Runtime>::get_equivalent_asset_amount(
					borrowed_asset,
					1, // USDT
					borrowed_amount,
				).unwrap_or_default();
	
				// Calculate the APY for borrowing
				let apy = pool.borrow_interest_rate().unwrap_or_default();

				// Accumulate total borrowed amount
				total_borrow = total_borrow.saturating_add(equivalent_borrowed_amount);
	
				// Create a BorrowedAsset
				let borrowed_asset_entry = BorrowedAsset {
					asset_info: AssetInfo {
						asset_id: borrowed_asset,
						asset_symbol: borrow_asset_symbol,
						asset_name: borrow_asset_name,
						decimals: borrow_asset_decimals,
						asset_icon: borrow_asset_icon,
						balance: borrow_balance,
					},
					apy,
					borrowed: borrowed_amount,
				};
				
				borrowed_assets.push(borrowed_asset_entry);
		
				// Handle Collateral Assets
				let collateral_balance = loan.collateral_balance;
		
				// Retrieve asset metadata for the borrowed asset
				let collateral_asset_name = pallet_assets::Pallet::<Runtime>::name(collateral_asset);
				let collateral_asset_symbol = pallet_assets::Pallet::<Runtime>::symbol(collateral_asset);
				let collateral_asset_decimals = pallet_assets::Pallet::<Runtime>::decimals(collateral_asset);
				let collateral_asset_icon = "<url>/dot.svg".as_bytes().to_vec(); // Mocked for now
	
				// Calculate equivalent collateral amount in USDT 
				let equivalent_collateral_amount = lending::Pallet::<Runtime>::get_equivalent_asset_amount(
					collateral_asset,
					1, // USDT
					collateral_balance,
				).unwrap_or_default();
	
				// Accumulate total collateral amount
				total_collateral = total_collateral.saturating_add(equivalent_collateral_amount);
	
				// Create a CollateralAsset
				let collateral_asset_entry = CollateralAsset {
					asset_info: AssetInfo {
						asset_id: collateral_asset,
						asset_symbol: collateral_asset_symbol,
						asset_name: collateral_asset_name,
						decimals: collateral_asset_decimals,
						asset_icon: collateral_asset_icon,
						balance: collateral_balance,
					},
				};
				
				collateral_assets.push(collateral_asset_entry);
			}
		
			// Return the list of borrowed assets, collateral assets, total borrow, and total collateral
			(borrowed_assets, collateral_assets, total_borrow, total_collateral)
		}
		
	}
}
