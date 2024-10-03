#![cfg_attr(not(feature = "std"), no_std)]

pub use borrow_repay::UserBorrow;
use frame_support::traits::tokens::fungibles::metadata::Inspect as MetadataInspect;
///! # The Lending pallet of Kylix
///!
///! ## Overview
///!
///! The Lending pallet is responsible for managing the lending pools and the treasury
/// operations. !
///! The lending pallet adopts a protocol similar to Compound V2 for its lending operations,
///! offering a pool-based approach to aggregate assets from all users.
///!
///! Interest rates adjust dynamically in response to the supply and demand conditions.
///! Additionally, for every lending positions a new token is minted, thus enabling the
/// transfer of ! ownership.
///!
///! Implemented Extrinsics:
///!
///! 0. create_lending_pool()
///! 1. activate_lending_pool()
///! 2. supply()
///! 3. withdraw()
///! 4. borrow()
///! 5. repay()
///! 6. claim_rewards()
///! 7. deactivate_lending_pool()
///! 8. update_pool_rate_model()
///! 9. update_pool_kink()
///!
///
/// TODO:
/// 1. rename the pallet to `lending` and the module to `lending`
/// 2. implement the `ManagerOrigin` type for reserve pool special operations
/// 3. implement tests for the lending logic
/// 4. implement the `WeightInfo` trait for the pallet
///
///! Use case
pub use frame_support::{
	pallet_prelude::*,
	serde, sp_runtime,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero},
		FixedPointNumber, FixedU128, Permill, SaturatedConversion,
	},
	traits::{
		fungible, fungibles,
		fungibles::{Create, Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		Time as MomentTime,
	},
	DefaultNoBound, PalletId,
};
pub use frame_system::pallet_prelude::*;
pub use interest_rate::InterestRateModel;
pub use pallet::*;
use scale_info::prelude::vec::Vec;
use serde::{Deserialize, Serialize};

/// Account Type Definition
pub type AccountOf<T> = <T as frame_system::Config>::AccountId;

/// Fungible Asset Id
pub type AssetIdOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<AccountOf<T>>>::AssetId;

/// Fungible Balance
pub type AssetBalanceOf<T> =
	<<T as Config>::Fungibles as fungibles::Inspect<AccountOf<T>>>::Balance;

/// Native Balance
pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<AccountOf<T>>>::Balance;
//type BalanceOf<T> = <T as currency::Config>::Balance;

pub type Timestamp = u64;
pub type Rate = FixedU128;
pub type Ratio = Permill;
pub type LendingPoolId = u32;

/// Total value of all deposits in USDT for a given account.
pub type TotalDeposit = u128;

/// Total value of all borrow assets in USDT for a given account.
pub type TotalBorrow = u128;

/// Total value of all collateral in USDT for a given account.
pub type TotalCollateral = u128;

pub const SECONDS_PER_YEAR: u64 = 365u64 * 24 * 60 * 60;

mod borrow_repay;
mod interest_rate;

#[cfg(test)]
pub(crate) mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct LendingPoolInfo {
	pub id: u32,
	pub asset_id: u32,
	pub asset: Vec<u8>,
	pub asset_decimals: u32,
	pub asset_icon: Vec<u8>,
	pub asset_symbol: Vec<u8>,
	pub collateral_q: u64,
	pub utilization: FixedU128,
	pub borrow_apy: FixedU128,
	pub borrow_apy_s: FixedU128,
	pub supply_apy: FixedU128,
	pub supply_apy_s: FixedU128,
	pub is_activated: bool,
	pub user_asset_balance: Option<u128>
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct AggregatedTotals {
	pub total_supply: u128,
	pub total_borrow: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct AssetInfo {
	pub asset_id: u32,
	pub asset_symbol: Vec<u8>,
	pub asset_name: Vec<u8>,
	pub decimals: u8,
	pub asset_icon: Vec<u8>,
	pub balance: u128,
}

/// Supplied asset definition. Used as response for rpc
#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct SuppliedAsset {
	#[serde(flatten)]
	pub asset_info: AssetInfo,
	pub apy: FixedU128,
	pub supplied: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct BorrowedAsset {
	#[serde(flatten)]
	pub asset_info: AssetInfo,
	pub apy: FixedU128,
	pub borrowed: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, Debug, TypeInfo)]
pub struct CollateralAsset {
	#[serde(flatten)]
	pub asset_info: AssetInfo,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config<AssetId = u32> {
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type NativeBalance: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::hold::Inspect<Self::AccountId>
			+ fungible::hold::Mutate<Self::AccountId>
			+ fungible::freeze::Inspect<Self::AccountId>
			+ fungible::freeze::Mutate<Self::AccountId>;

		/// Type to access the Assets Pallet.
		type Fungibles: fungibles::Inspect<Self::AccountId, Balance = BalanceOf<Self>, AssetId = u32>
			+ fungibles::Mutate<Self::AccountId>
			+ fungibles::Create<Self::AccountId>;

		/// The origin which can add or remove LendingPools and update LendingPools TODO
		// type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		type Time: MomentTime;
	}

	/// The AssetPool definition. Used as the KEY in the lending pool storage
	#[derive(
		Clone,
		Encode,
		Decode,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
		PartialOrd,
		DefaultNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct AssetPool<T: Config> {
		asset: AssetIdOf<T>,
	}
	impl<T: Config> AssetPool<T> {
		pub fn from(asset: AssetIdOf<T>) -> Self {
			AssetPool { asset }
		}
	}

	/// Definition of the Lending Pool Reserve Entity
	///
	/// A struct to hold the LendingPool and all its properties,
	/// used as Value in the lending pool storage
	///
	/// Current interest rate model being used is the "Jump Model"
	#[derive(
		Clone,
		Encode,
		Decode,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
		PartialOrd,
		DefaultNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct LendingPool<T: Config> {
		pub id: LendingPoolId,           // the lending pool id
		pub lend_token_id: AssetIdOf<T>, // the lending token id

		pub reserve_balance: AssetBalanceOf<T>, // the reserve supplied to the lending pool
		pub borrowed_balance: AssetBalanceOf<T>, // the borrowed balance from the lending pool

		pub activated: bool, // is the pool active or in pending state?

		// defined by pool creator, but hardcoded default interest rate model for the time being
		pub interest_model: InterestRateModel,

		// 'reserve_factor' determines the split between what the depositors enjoy versus what
		// flows into Kylix's treasury.
		pub reserve_factor: Ratio,

		pub exchange_rate: Ratio, // defined by user, 20% Default exchange rate

		pub collateral_factor: Ratio,     // The secure collateral ratio
		pub liquidation_threshold: Ratio, // defined by user, 75% as default

		pub borrow_rate: Ratio, // the borrow rate of the pool
		pub supply_rate: Ratio, // the supply rate of the pool

		pub last_accrued_interest_at: Timestamp, /* the timestamp of the last calculation of
		                                          * accrued interest */
		pub borrow_index: Rate, // accumulator of the total earned interest rate
		pub supply_index: Rate, // accumulator of the total earned interest rate
	}
	impl<T: Config> LendingPool<T> {
		// let's create a default reserve lending pool
		pub fn from(
			id: LendingPoolId,
			lend_token_id: AssetIdOf<T>,
			balance: AssetBalanceOf<T>,
		) -> Result<Self, Error<T>> {
			let mut pool = LendingPool {
				id,
				lend_token_id,

				reserve_balance: balance,
				borrowed_balance: AssetBalanceOf::<T>::zero(),

				activated: false,

				interest_model: InterestRateModel::default(),
				reserve_factor: Ratio::from_percent(10), // Default reserve factor at 10%
				borrow_rate: Ratio::from_percent(20),    // Default 0.20 as borrow rate ratio

				collateral_factor: Ratio::from_percent(50), // Default collateral factor at 50%
				liquidation_threshold: Ratio::from_percent(80), // Default liquidation at 80%

				supply_rate: Ratio::zero(),
				exchange_rate: Ratio::zero(),
				last_accrued_interest_at: Pallet::<T>::now_in_seconds(),
				borrow_index: Rate::one(),
				supply_index: Rate::one(),
			};
			pool.update_indexes()?;
			Ok(pool)
		}

		///
		/// Ut -> utilisation ratio calculated as
		/// 	borrowed_balance / (borrowed_balance + reserve_balance)
		pub fn utilisation_ratio(&self) -> Result<Ratio, Error<T>> {
			if self.is_empty() {
				return Ok(Ratio::zero());
			}

			let denominator = self
				.borrowed_balance
				.checked_add(&self.reserve_balance)
				.ok_or(Error::<T>::OverflowError)?;

			Ok(Ratio::from_rational(self.borrowed_balance, denominator))
		}

		pub fn borrow_interest_rate(&self) -> Result<Rate, Error<T>> {
			if self.borrowed_balance.is_zero() || self.reserve_balance.is_zero() {
				return Ok(Rate::zero());
			}

			let utilisation_ratio = self.utilisation_ratio()?;
			let utilisation_ratio: Rate = utilisation_ratio.into();

			self.interest_model
				.calculate_cosine_interest(utilisation_ratio)
				.map_err(|_| Error::<T>::OverflowError.into())
		}

		///
		/// The SUPPLY interest rate model calculated as
		///
		/// (borrow_rate * utilization_ratio) * (1 - reserve_factor)
		pub fn supply_interest_rate(&self) -> Result<Rate, Error<T>> {
			//
			let borrow_rate = self.borrow_interest_rate()?;
			let utilisation_ratio = self.utilisation_ratio()?;

			let reserved = Permill::from_percent(100)
				.checked_sub(&self.reserve_factor)
				.ok_or(Error::<T>::OverflowError)?;

			let res = borrow_rate
				.checked_mul(&utilisation_ratio.into())
				.ok_or(Error::<T>::OverflowError)?
				.checked_mul(&reserved.into())
				.ok_or(Error::<T>::OverflowError)?;

			Ok(res)
		}

		/// self-explanatory helper methods
		pub fn is_empty(&self) -> bool {
			self.reserve_balance.cmp(&BalanceOf::<T>::zero()).is_eq()
		}
		pub fn is_active(&self) -> bool {
			self.activated == true
		}

		/// Calculates scaled balance as
		/// scaled_balance = balance / supply_index
		pub fn scaled_supply_balance(
			&self,
			deposit: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let scaled_balance = FixedU128::from_inner(deposit.saturated_into())
				.checked_div(&self.supply_index)
				.ok_or(Error::<T>::OverflowError)?
				.into_inner()
				.saturated_into();
			Ok(scaled_balance)
		}

		/// Calculates scaled balance for borrow case as
		/// scaled_balance = balance / borrow_index
		pub fn scaled_borrow_balance(
			&self,
			borrowed_balance: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let scaled_balance = FixedU128::from_inner(borrowed_balance.saturated_into())
				.checked_div(&self.borrow_index)
				.ok_or(Error::<T>::OverflowError)?
				.into_inner()
				.saturated_into();
			Ok(scaled_balance)
		}

		/// Calculates linear interest as follows
		/// 	rate_per_second = rate / SECONDS_PER_YEAR
		/// 	duration = now - last_updated_timestamp
		/// 	rate = 1 + rate_per_second * duration
		/// # Arguments
		/// rate: Annual supply interest rate
		fn calculate_linear_interest(&self) -> Result<Rate, Error<T>> {
			let dur: u64 = Pallet::<T>::now_in_seconds()
				.checked_sub(self.last_accrued_interest_at)
				.ok_or(Error::<T>::OverflowError)?;
			let rate_factor = self
				.supply_interest_rate()?
				.checked_mul(&FixedU128::from(dur as u128))
				.ok_or(Error::<T>::OverflowError)?;
			let accumulated_rate = rate_factor
				.checked_div(&(SECONDS_PER_YEAR as u128).into())
				.ok_or(Error::<T>::OverflowError)?
				.checked_add(&FixedU128::one())
				.ok_or(Error::<T>::OverflowError)?;
			Ok(accumulated_rate)
		}

		fn exp_fixed_u128(&self, x: FixedU128) -> Result<FixedU128, Error<T>> {
			let mut sum = FixedU128::one();
			let mut term = FixedU128::one();
			let mut n = 1u128;

			loop {
				term = term
					.checked_mul(&x)
					.ok_or(Error::<T>::OverflowError)?
					.checked_div(&FixedU128::from(n))
					.ok_or(Error::<T>::OverflowError)?;
				sum = sum.checked_add(&term).ok_or(Error::<T>::OverflowError)?;

				if term < FixedU128::from_inner(1_000_000) {
					break;
				}

				n += 1;
				if n > 20 {
					// Limit the number of iterations to prevent infinite loops
					break;
				}
			}
			Ok(sum)
		}

		/// Calculate compounded interest
		/// Borrow interest compounds every second. This is achieved by using Taylor Series
		/// Approximation
		fn calculate_compunded_interest(&self) -> Result<Rate, Error<T>> {
			let rate = self
				.borrow_interest_rate()?
				.checked_div(&(SECONDS_PER_YEAR as u128).into())
				.ok_or(Error::<T>::OverflowError)?;
			let t = Pallet::<T>::now_in_seconds()
				.checked_sub(self.last_accrued_interest_at)
				.ok_or(Error::<T>::OverflowError)?;
			// Compute x = rate * t
			let x =
				rate.checked_mul(&FixedU128::from(t as u128)).ok_or(Error::<T>::OverflowError)?;

			// Compute interest = exp(x)
			let interest = self.exp_fixed_u128(x)?;

			Ok(interest)
		}

		fn update_supply_index(&mut self) -> Result<(), Error<T>> {
			let incr = self.calculate_linear_interest()?;
			let new_index =
				self.supply_index.checked_mul(&incr).ok_or(Error::<T>::OverflowError)?;
			self.supply_index = new_index;
			Ok(())
		}

		fn update_borrow_index(&mut self) -> Result<(), Error<T>> {
			let incr = self.calculate_compunded_interest()?;
			let new_index =
				self.borrow_index.checked_mul(&incr).ok_or(Error::<T>::OverflowError)?;
			self.borrow_index = new_index;
			Ok(())
		}

		pub fn update_indexes(&mut self) -> Result<(), Error<T>> {
			if self.last_accrued_interest_at < Pallet::<T>::now_in_seconds() {
				self.update_supply_index()?;
				self.update_borrow_index()?;
				self.last_accrued_interest_at = Pallet::<T>::now_in_seconds();
			}
			Ok(())
		}

		/// Calculates accrued deposit as
		/// accrued_deposit = deposit * supply_index
		pub fn accrued_deposit(
			&self,
			balance: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let a_deposit = FixedU128::from_inner(balance.saturated_into())
				.checked_mul(&self.supply_index)
				.ok_or(Error::<T>::OverflowError)?
				.into_inner()
				.saturated_into();
			Ok(a_deposit)
		}

		/// Update pool: move assets from reserved_balance to borrowed_balance
		pub fn move_asset_on_borrow(&mut self, balance: AssetBalanceOf<T>) -> Result<(), Error<T>> {
			self.reserve_balance =
				self.reserve_balance.checked_sub(&balance).ok_or(Error::<T>::OverflowError)?;
			self.borrowed_balance =
				self.borrowed_balance.checked_add(&balance).ok_or(Error::<T>::OverflowError)?;
			Ok(())
		}

		/// Update pool: move assets from borrowed_balance reserved_balance on repayment
		pub fn move_asset_on_repay(
			&mut self,
			pay: AssetBalanceOf<T>,
			principal_reduction: AssetBalanceOf<T>,
		) -> Result<(), Error<T>> {
			self.borrowed_balance = self
				.borrowed_balance
				.checked_sub(&principal_reduction)
				.ok_or(Error::<T>::OverflowError)?;
			self.reserve_balance =
				self.reserve_balance.checked_add(&pay).ok_or(Error::<T>::OverflowError)?;
			Ok(())
		}

		/// Calculate the loan amount
		/// max_loan_amount = collateral_balance * collatoral_factor
		pub fn max_borrow_amount(
			&self,
			collateral_balance: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let factor: Rate = self.collateral_factor.into();
			let max_loan_amount = FixedU128::from_inner(collateral_balance.saturated_into())
				.checked_mul(&factor)
				.ok_or(Error::<T>::OverflowError)?
				.into_inner()
				.saturated_into();
			Ok(max_loan_amount)
		}

		/// Calculate the repayable amount including accrued interest
		/// repayable_amount = borrowed_balance * (current_borrow_index /
		/// borrow_index_at_borrow_time)
		pub fn repayable_amount(
			&self,
			loan: &UserBorrow<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let index_ratio = self
				.borrow_index
				.checked_div(&loan.borrow_index_at_borrow_time)
				.ok_or(Error::<T>::OverflowError)?;

			let borrowed_balance_u128 = loan.borrowed_balance.saturated_into::<u128>();

			let repayable_amount_u128 = index_ratio
				.checked_mul(&FixedU128::from(borrowed_balance_u128))
				.ok_or(Error::<T>::OverflowError)?
				.into_inner() / FixedU128::accuracy();

			let repayable_amount = repayable_amount_u128.saturated_into::<AssetBalanceOf<T>>();

			Ok(repayable_amount)
		}
	}

	/// Kylix runtime storage items
	///
	/// Lending pools defined for the assets
	///
	/// StorageMap AssetPool { AssetId } => LendingPool { PoolId, Balance }
	#[pallet::storage]
	#[pallet::getter(fn reserve_pools)]
	pub type LendingPoolStorage<T> = StorageMap<_, Blake2_128Concat, AssetPool<T>, LendingPool<T>>;

	// Now we need to define the properties of the underlying asset used in the lending pool
	#[derive(
		Clone,
		Encode,
		Decode,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
		PartialOrd,
		DefaultNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct UnderlyingAsset<T: Config> {
		pub underlying_asset_id: AssetIdOf<T>, /* Mapping of lend_token id to underlying
		                                        * currency id */
		pub last_accrued_interest: Timestamp, /* the timestamp of the last calculation of
		                                       * accrued interest */

		pub total_borrowed: AssetBalanceOf<T>, // the total amount borrowed from the pool
		pub total_supply: AssetBalanceOf<T>,   // the total amount supplied to the pool

		pub borrow_index: Rate, // accumulator of the total earned interest rate
		pub exchange_rate: Rate, /* the exchange rate from the associated lend token to the
		                         * underlying asset */
		pub borrow_rate: Rate, // the current borrow rate
		pub supply_rate: Rate, // the current supply rate

		pub utilization_rate: Rate, // the current utilization rate

		pub reward_supply_speed: AssetBalanceOf<T>, // the current reward supply speed
		pub reward_borrow_speed: AssetBalanceOf<T>, // the current reward borrow speed
		pub reward_accrued: AssetBalanceOf<T>,      // the current reward accrued
	}

	//  The accrued supply_index of the supplier
	#[derive(
		Clone,
		Encode,
		Decode,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
		PartialOrd,
		DefaultNoBound,
	)]
	pub struct SupplyIndex {
		pub supply_index: Rate,
		pub last_accrued_interest_at: Timestamp,
	}

	impl SupplyIndex {
		pub fn from(supply_index: Rate, last_accrued_interest_at: Timestamp) -> Self {
			Self { supply_index, last_accrued_interest_at }
		}
	}

	/// Kylix runtime storage items
	///
	/// Lending pools Assets Properties
	///
	/// StorageMap CurrencyId<T> { AssetId } => LendingAsset { PoolId, Balance }
	///
	/// The timestamp of the last calculation of accrued interest
	#[pallet::storage]
	#[pallet::getter(fn last_accrued_interest_time)]
	pub type UnderlyingAssetStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, UnderlyingAsset<T>, ValueQuery>;

	/// The minimum (starting) and maximum exchange rate allowed for a market.
	#[pallet::storage]
	#[pallet::getter(fn max_exchange_rate)]
	pub type MinMaxExchangeRate<T: Config> = StorageValue<_, (Rate, Rate), ValueQuery>;

	/// The accrued supply_index of accounts for assets
	#[pallet::storage]
	pub type SupplyIndexStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, (AccountOf<T>, AssetIdOf<T>), SupplyIndex, ValueQuery>;

	/// The borrow status of accounts
	/// (AccountId, borrowed_asset_id, collateral_asset_id) => UserBorrow details
	#[pallet::storage]
	pub type Borrows<T: Config> =
		StorageMap<_, Blake2_128Concat, (AccountOf<T>, AssetIdOf<T>, AssetIdOf<T>), UserBorrow<T>>;

	/// The storage to hold prices of assets w.r.t. other assets
	/// This is the dummy storage, ideally this functionality would be implemented in a dedicated
	/// pallet store (asset_id1, asset_id2) => FixedU128
	#[pallet::storage]
	pub type AssetPrices<T: Config> =
		StorageMap<_, Blake2_128Concat, (AssetIdOf<T>, AssetIdOf<T>), FixedU128, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub _marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let max_exchange_rate: u128 = 1_000_000_000_000_000_000; // 1
			let min_exchange_rate: u128 = 20_000_000_000_000_000; // 0.02

			let rmax = Rate::from_inner(max_exchange_rate);
			let rmin = Rate::from_inner(min_exchange_rate);

			MinMaxExchangeRate::<T>::put((rmin, rmax));
		}
	}

	/// Events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		DepositSupplied { who: T::AccountId, asset: AssetIdOf<T>, balance: BalanceOf<T> },
		DepositWithdrawn { who: T::AccountId, balance: BalanceOf<T> },
		DepositBorrowed { who: T::AccountId, balance: BalanceOf<T> },
		DepositRepaid { who: T::AccountId, balance: BalanceOf<T> },
		RewardsClaimed { who: T::AccountId, balance: BalanceOf<T> },
		LendingPoolAdded { who: T::AccountId, asset: AssetIdOf<T> },
		LendingPoolRemoved { who: T::AccountId },
		LendingPoolActivated { who: T::AccountId, asset: AssetIdOf<T> },
		LendingPoolDeactivated { who: T::AccountId, asset: AssetIdOf<T> },
		LendingPoolRateModelUpdated { who: T::AccountId, asset: AssetIdOf<T> },
		LendingPoolKinkUpdated { who: T::AccountId, asset: AssetIdOf<T> },
		LPTokenMinted { who: T::AccountId, asset: AssetIdOf<T>, balance: AssetBalanceOf<T> },
		AssetPriceAdded { asset_1: AssetIdOf<T>, asset_2: AssetIdOf<T>, price: FixedU128 },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Lending Pool does not exist
		LendingPoolDoesNotExist,
		/// Lending Pool already exists
		LendingPoolAlreadyExists,
		/// Lending Pool already activated
		LendingPoolAlreadyActivated,
		/// Lending Pool already deactivated
		LendingPoolAlreadyDeactivated,
		/// Lending Pool is not active or has been deprecated
		LendingPoolNotActive,
		/// The balance amount to supply is not valid
		InvalidLiquiditySupply,
		/// The balance amount to withdraw is not valid
		InvalidLiquidityWithdrawal,
		/// The user has not enough liquidity
		NotEnoughLiquiditySupply,
		/// The user wants to withdraw more than allowed!
		NotEnoughEligibleLiquidityToWithdraw,
		/// Lending Pool is empty
		LendingPoolIsEmpty,
		/// The classic Overflow Error
		OverflowError,
		/// The ID already exists
		IdAlreadyExists,
		/// The user has not enough collateral assets
		NotEnoughCollateral,
		/// The Loan being repaid does not exist
		LoanDoesNotExists,
		/// Price of the asset can not be zero
		InvalidAssetPrice,
		/// The price of the asset is not available
		AssetPriceNotSet,
		/// Division by zero
		DivisionByZero,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The `create_lending_pool` function allows a user to Create a new reserve and then
		/// supply it with some liquidity. Given an asset and its amount, it creates a
		/// new lending pool, if it does not already exist, and adds the provided liquidity
		///
		/// The user will receive LP tokens in return in ratio.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user that
		///   creates the lending pool and add some liquidity.
		/// * `id`: AssetIdOf<T> - The pool id, provided by the user
		/// * `asset` - The identifier for the type of asset that the user wants to provide.
		/// * `balance` - The amount of `asset` that the user is providing.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		/// * If the provided assets do not exist.
		/// * If `amount` is 0 or less.
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflow or
		///   underflow
		///
		/// # Events
		///
		/// If the function succeeds, it triggers two events:
		///
		/// * `LendingPoolAdded(who, asset_a)` if a new lending pool was created.
		/// * `DepositSupplied(who, asset_a, amount_a)` after the liquidity has been successfully
		///   added.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn create_lending_pool(
			origin: OriginFor<T>,
			id: LendingPoolId,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_create_lending_pool(&who, id, asset, balance)?;
			Self::deposit_event(Event::LendingPoolAdded { who: who.clone(), asset });
			Self::deposit_event(Event::DepositSupplied { who, asset, balance });
			Ok(())
		}

		/// The `activate_lending_pool` function allows a user to activate a lending pool that is
		/// not empty. Once a liquidity pool gets activated supplies operations can be performed
		/// otherwise only withdrawals.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		///  that creates the lending pool and add some liquidity.
		/// * `asset` - The identifier for the type of asset that the user wants to provide.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		/// * If the provided assets do not exist.
		/// * If the pool does not exist.
		/// * If the pool is already activated.
		/// * If the pool is empty.
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `LendingPoolActivated(who, asset_a)` if the lending pool was activated.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		pub fn activate_lending_pool(origin: OriginFor<T>, asset: AssetIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_activate_lending_pool(asset)?;
			Self::deposit_event(Event::LendingPoolActivated { who, asset });
			Ok(())
		}

		/// The `supply` function allows a user to supply liquidity to a lending pool.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		/// that creates the lending pool and add some liquidity.
		/// * `asset` - The identifier for the type of asset that the user wants to provide.
		/// * `balance` - The amount of `asset` that the user is providing.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		/// * If the provided assets do not exist.
		/// * If the pool does not exist.
		/// * If the pool is not active.
		/// * If the user has not enough liquidity to supply.
		/// * If the balance amount to supply is not valid.
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflow or
		///  underflow
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `DepositSupplied(who, asset, balance)` if the lending pool has been supplied.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::default())]
		pub fn supply(
			origin: OriginFor<T>,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_supply(&who, asset, balance)?;
			Self::deposit_event(Event::DepositSupplied { who, asset, balance });
			Ok(())
		}

		/// The `withdraw` function allows a user to withdraw liquidity from a lending pool.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		/// that creates the lending pool and add some liquidity.
		/// * `asset` - The identifier for the type of asset that the user wants to provide.
		/// * `balance` - The amount of `asset` that the user is providing.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		/// * If the provided assets do not exist.
		/// * If the pool does not exist.
		/// * If the pool is not active.
		/// * If the user has not enough liquidity to supply.
		/// * If the balance amount to supply is not valid.
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflow or
		/// underflow
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `DepositWithdrawn(who, balance)` if the lending pool was activated.
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::default())]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_withdrawal(&who, asset, balance)?;
			Self::deposit_event(Event::DepositWithdrawn { who, balance });
			Ok(())
		}

		/// The `borrow` function allows a user to borrow liquidity from a lending pool.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		/// that creates the lending pool and add some liquidity.
		/// * `asset` - The identifier for the type of asset that the user wants to provide.
		/// * `balance` - The amount of `asset` that the user is providing.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		/// * If the provided assets do not exist.
		/// * If the pool does not exist.
		/// * If the pool is not active.
		/// * If the user has not enough liquidity to supply.
		/// * If the balance amount to supply is not valid.
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflow or
		/// underflow
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `DepositBorrowed(who, balance)` if the lending pool was activated.
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::default())]
		pub fn borrow(
			origin: OriginFor<T>,
			asset: AssetIdOf<T>,
			balance: AssetBalanceOf<T>,
			collateral_asset: AssetIdOf<T>,
			collateral_balance: AssetBalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_borrow(&who, asset, balance, collateral_asset, collateral_balance)?;
			Self::deposit_event(Event::DepositBorrowed { who, balance });
			Ok(())
		}

		/// The `repay` function allows a user to repay liquidity from a lending pool.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		/// that creates the lending pool and add some liquidity.
		/// * `asset` - The identifier for the type of asset that the user wants to provide.
		/// * `balance` - The amount of `asset` that the user is providing.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		/// * If the provided assets do not exist.
		/// * If the pool does not exist.
		/// * If the pool is not active.
		/// * If the user has not enough liquidity to supply.
		/// * If the balance amount to supply is not valid.
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflow or
		/// underflow
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `DepositRepaid(who, balance)` if the lending pool was activated.
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::default())]
		pub fn repay(
			origin: OriginFor<T>,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
			collateral_asset: AssetIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_repay(&who, asset, balance, collateral_asset)?;
			Self::deposit_event(Event::DepositRepaid { who, balance });
			Ok(())
		}

		/// The `claim_rewards` function allows a user to claim their rewards.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		///   claiming rewards.
		/// * `balance` - The amount of rewards to be claimed.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `RewardsClaimed { who, balance }`: Notifies the system that rewards have been claimed
		///   by a user.
		#[pallet::call_index(6)]
		#[pallet::weight(Weight::default())]
		pub fn claim_rewards(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::RewardsClaimed { who, balance });
			Ok(())
		}

		/// The `deactivate_lending_pool` function allows a user to deactivate a lending pool that
		/// is not empty. Once a liquidity pool gets deactivated supplies operations can not be
		/// performed otherwise only withdrawals.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		/// that creates the lending pool and add some liquidity.
		/// * `asset` - The identifier for the type of asset that the user wants to provide.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		/// * If the provided assets do not exist.
		/// * If the pool does not exist.
		/// * If the pool is already deactivated.
		/// * If the pool is empty.
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `LendingPoolDeactivated(who, asset_a)` if the lending pool was deactivated.
		#[pallet::call_index(7)]
		#[pallet::weight(Weight::default())]
		pub fn deactivate_lending_pool(
			origin: OriginFor<T>,
			asset: AssetIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::LendingPoolDeactivated { who, asset });
			Ok(())
		}

		/// The `update_pool_rate_model` function allows a user to update the rate model of a
		/// lending pool.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		///   updating the rate model.
		/// * `asset` - The identifier for the type of asset associated with the lending pool.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `LendingPoolRateModelUpdated { who, asset }`: Notifies the system that the rate model
		///   of a lending pool was updated by a user.
		#[pallet::call_index(8)]
		#[pallet::weight(Weight::default())]
		pub fn update_pool_rate_model(origin: OriginFor<T>, asset: AssetIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::LendingPoolRateModelUpdated { who, asset });
			Ok(())
		}

		/// The `update_pool_kink` function allows a user to update the kink of a lending pool.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		///   updating the kink.
		/// * `asset` - The identifier for the type of asset associated with the lending pool.
		///
		/// # Errors
		///
		/// This function will return an error in the following scenarios:
		///
		/// * If the origin is not signed (i.e., the function was not called by a user).
		///
		/// # Events
		///
		/// If the function succeeds, it triggers an event:
		///
		/// * `LendingPoolKinkUpdated { who, asset }`: Notifies the system that the kink of a
		///   lending pool was updated by a user.
		#[pallet::call_index(9)]
		#[pallet::weight(Weight::default())]
		pub fn update_pool_kink(origin: OriginFor<T>, asset: AssetIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::LendingPoolKinkUpdated { who, asset });
			Ok(())
		}

		/// Sets the price of one asset in terms of another asset.
		///
		/// The `set_asset_price` extrinsic allows a user to specify the relative price of one asset
		/// (`asset_1`) in terms of another asset (`asset_2`).
		///
		/// # Parameters
		/// - `origin`: The transaction origin. This must be a signed extrinsic.
		/// - `asset_1`: The identifier for the first asset. This is the asset whose price is being
		///   set.
		/// - `asset_2`: The identifier for the second asset. This is the asset relative to which
		///   the price is measured.
		/// - `price`: The price of `asset_1` in terms of `asset_2`. This must be a non-zero value
		///   to avoid errors.
		///
		/// # Events
		/// - `AssetPriceAdded { asset_1, asset_2, price }`: This event is emitted after the price
		///   is successfully set. It contains the asset identifiers and the new price.
		///
		/// # Errors
		/// - `InvalidAssetPrice`: This error is thrown if the `price` parameter is zero.
		///
		/// # Note this should be moved to a new pallet `prices`
		#[pallet::call_index(10)]
		#[pallet::weight(Weight::default())]
		pub fn set_asset_price(
			origin: OriginFor<T>,
			asset_1: AssetIdOf<T>,
			asset_2: AssetIdOf<T>,
			price: FixedU128,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			// price should not be zero
			ensure!(price > FixedU128::zero(), Error::<T>::InvalidAssetPrice);

			AssetPrices::<T>::set((asset_1, asset_2), Some(price));

			// Emit an event.
			Self::deposit_event(Event::AssetPriceAdded { asset_1, asset_2, price });

			Ok(())
		}
	}

	// the main logic of the pallet
	impl<T: Config> Pallet<T> {
		// This method creates a NEW lending pool and mints LP tokens back to the user.
		// At this moment, the user is the first liquidity provider
		// The pool must not exist and the user must have enough liquidity to supply.
		pub fn do_create_lending_pool(
			who: &T::AccountId,
			id: LendingPoolId,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			// First, let's check the balance amount is valid
			ensure!(balance > BalanceOf::<T>::zero(), Error::<T>::InvalidLiquiditySupply);

			// Second, let's check if the user has enough liquidity
			let user_balance = T::Fungibles::balance(asset.clone(), who);
			ensure!(user_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			// Now let's check if the pool is already existing, before creating a new one.
			let asset_pool = AssetPool::<T>::from(asset);
			ensure!(
				!LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolAlreadyExists
			);

			// make sure id does not exist already
			ensure!(!T::Fungibles::asset_exists(id.clone()), Error::<T>::IdAlreadyExists);

			// Now we can safely create and store our lending pool with an initial balance...
			let asset_pool = AssetPool::from(asset);
			let lending_pool = LendingPool::<T>::from(id, asset, balance)?;

			LendingPoolStorage::<T>::set(&asset_pool, Some(lending_pool.clone()));

			// Transfer the tokens (asset) from the users account into pallet account
			T::Fungibles::transfer(
				asset.clone(),
				who,
				&Self::account_id(),
				balance,
				Preservation::Expendable,
			)?;

			// create liquidity token
			T::Fungibles::create(id.clone(), Self::account_id(), true, One::one())?;

			let scaled_minted_tokens = lending_pool.scaled_supply_balance(balance)?;
			// mints the lp tokens into the users account
			Self::update_and_mint(who, asset, id, scaled_minted_tokens, lending_pool.supply_index)?;

			Self::deposit_event(Event::LPTokenMinted {
				who: who.clone(),
				asset: id,
				balance: scaled_minted_tokens,
			});
			Ok(())
		}

		// This method activates an existing lending pool that is not empty.
		// Once a liquidity pool gets activated supplies operations can be performed
		// otherwise only withdrawals.
		pub fn do_activate_lending_pool(asset: AssetIdOf<T>) -> DispatchResult {
			// let's check if our pool does exist before activating it
			let asset_pool = AssetPool::<T>::from(asset);
			let mut pool = LendingPoolStorage::<T>::get(&asset_pool)
				.ok_or_else(|| DispatchError::from(Error::<T>::LendingPoolDoesNotExist))?;

			// let's check if our pool is actually already active and balance > 0
			ensure!(pool.is_active() == false, Error::<T>::LendingPoolAlreadyActivated);
			ensure!(!pool.is_empty(), Error::<T>::LendingPoolIsEmpty);

			// ok now we can activate it
			pool.activated = true;
			LendingPoolStorage::set(&asset_pool, Some(pool));
			Ok(())
		}

		// This method supplies liquidity to a lending pool and mints LP tokens back to the user.
		// The pool must be active and the user must have enough liquidity to supply.
		pub fn do_supply(
			who: &T::AccountId,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			// First, let's check the balance amount to supply is valid
			ensure!(balance > BalanceOf::<T>::zero(), Error::<T>::InvalidLiquiditySupply);

			// Second, let's check if the user has enough liquidity tp supply
			let user_balance = T::Fungibles::balance(asset.clone(), who);
			ensure!(user_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			// let's check if our pool does exist
			let asset_pool = AssetPool::<T>::from(asset);
			let mut pool = LendingPoolStorage::<T>::get(&asset_pool)
				.ok_or_else(|| DispatchError::from(Error::<T>::LendingPoolDoesNotExist))?;

			// let's ensure that the lending pool is active
			ensure!(pool.is_active() == true, Error::<T>::LendingPoolNotActive);

			// Update pool supply index
			pool.update_indexes()?;
			pool.reserve_balance =
				pool.reserve_balance.checked_add(&balance).ok_or(Error::<T>::OverflowError)?;

			// Transfers the tokens (asset) from the users account into pallet account
			T::Fungibles::transfer(
				asset.clone(),
				who,
				&Self::account_id(),
				balance,
				Preservation::Expendable,
			)?;

			let scaled_minted_tokens = pool.scaled_supply_balance(balance)?;
			let current_supply_index = pool.supply_index;
			Self::update_and_mint(who, asset, pool.id, scaled_minted_tokens, current_supply_index)?;

			// let's update the balances of the pool now
			LendingPoolStorage::<T>::set(&asset_pool, Some(pool));

			Ok(())
		}

		/// This method allows a user to withdraw liquidity from a lending pool.
		/// The pool can be deactivated or not, but the user must have enough LP tokens to withdraw.
		/// This method withdraw some liquidity from a liquidy pool and burns LP tokens of the user
		pub fn do_withdrawal(
			who: &T::AccountId,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			// First, let's check the balance amount to supply is valid
			ensure!(balance > BalanceOf::<T>::zero(), Error::<T>::InvalidLiquidityWithdrawal);

			// let's check if our pool does exist
			let asset_pool = AssetPool::<T>::from(asset);
			let mut pool = LendingPoolStorage::<T>::get(&asset_pool)
				.ok_or_else(|| DispatchError::from(Error::<T>::LendingPoolDoesNotExist))?;

			// let's check if the pool has enough liquidity
			ensure!(pool.reserve_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			// Update pool's indexes
			pool.update_indexes()?;

			// let's check if the user is actually eligible to withdraw!
			let scaled_lp_tokens = T::Fungibles::balance(pool.id.clone(), &who);
			let eligible_lp_tokens = pool.accrued_deposit(scaled_lp_tokens)?;
			ensure!(
				eligible_lp_tokens >= balance,
				Error::<T>::NotEnoughEligibleLiquidityToWithdraw
			);

			// Transfer the asset to the user
			T::Fungibles::transfer(
				asset.clone(),
				&Self::account_id(),
				who,
				balance,
				Preservation::Preserve,
			)?;

			// burn the LP asset
			let burnable_amount = pool.scaled_supply_balance(balance)?;
			T::Fungibles::burn_from(
				pool.id,
				who,
				burnable_amount,
				Precision::Exact,
				Fortitude::Force,
			)?;
			pool.reserve_balance =
				pool.reserve_balance.checked_sub(&balance).ok_or(Error::<T>::OverflowError)?;

			// let's update the balances of the pool now
			LendingPoolStorage::<T>::set(&asset_pool, Some(pool));

			Ok(())
		}

		///
		fn do_borrow(
			who: &T::AccountId,
			asset: AssetIdOf<T>,
			balance: AssetBalanceOf<T>,
			collateral_asset: AssetIdOf<T>,
			collateral_balance: AssetBalanceOf<T>,
		) -> DispatchResult {
			// First, let's check the balance amount to supply is valid
			ensure!(balance > BalanceOf::<T>::zero(), Error::<T>::InvalidLiquidityWithdrawal);
			ensure!(
				collateral_balance > AssetBalanceOf::<T>::zero(),
				Error::<T>::InvalidLiquidityWithdrawal
			);

			// let's check if our pool does exist
			let asset_pool = AssetPool::<T>::from(asset);
			let mut pool = LendingPoolStorage::<T>::get(&asset_pool)
				.ok_or_else(|| DispatchError::from(Error::<T>::LendingPoolDoesNotExist))?;
			let user_collateral_balance = T::Fungibles::reducible_balance(
				collateral_asset,
				who,
				Preservation::Preserve,
				Fortitude::Polite,
			);
			ensure!(
				user_collateral_balance >= collateral_balance,
				Error::<T>::NotEnoughLiquiditySupply
			);

			// let's check if the pool is active
			ensure!(pool.is_active() == true, Error::<T>::LendingPoolNotActive);

			// let's check if the pool has enough liquidity
			ensure!(pool.reserve_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			// Update pool's indexes
			pool.update_indexes()?;

			// check sufficiency of collateral asset
			// get collateral asset value in terms of borrow-asset
			let equivalent_asset_balance =
				Self::get_equivalent_asset_amount(asset, collateral_asset, collateral_balance)?;
			// get eligible borrow quantity based on reserve_factor
			let eligible_asset_amount = pool.max_borrow_amount(equivalent_asset_balance)?;
			// error if borrow is more than eligibility
			ensure!(eligible_asset_amount >= balance, Error::<T>::NotEnoughCollateral);

			// Save scaled balance as per current borrow_index
			let scaled_balance = pool.scaled_borrow_balance(balance)?;

			let borrow: UserBorrow<T> = UserBorrow {
				borrowed_asset: asset,
				borrowed_balance: scaled_balance,
				collateral_asset,
				collateral_balance,
				borrow_index_at_borrow_time: pool.borrow_index,
			};

			Borrows::<T>::try_mutate(
				(who, asset, collateral_asset),
				|maybe_borrow| -> DispatchResult {
					if let Some(borrow_record) = maybe_borrow {
						// Update the existing record.
						borrow_record.increase_borrow(&borrow)?;
					} else {
						// The entry does not exist, so we assign `Some(borrow)` to it to store the
						// new value
						*maybe_borrow = Some(borrow);
					}
					Ok(())
				},
			)?;

			// Update pool: transfer asset from reserved_balance to borrowed_balance
			pool.move_asset_on_borrow(balance)?;

			LendingPoolStorage::<T>::set(&asset_pool, Some(pool));

			// Transfer the asset to the user
			T::Fungibles::transfer(
				asset.clone(),
				&Self::account_id(),
				who,
				balance,
				Preservation::Preserve,
			)?;

			// Transfer the collateral to the pallet
			T::Fungibles::transfer(
				collateral_asset.clone(),
				who,
				&Self::account_id(),
				collateral_balance,
				Preservation::Preserve,
			)?;

			Ok(())
		}

		fn do_repay(
			who: &T::AccountId,
			asset: AssetIdOf<T>,
			balance: AssetBalanceOf<T>,
			collateral_asset: AssetIdOf<T>,
		) -> DispatchResult {
			ensure!(balance > BalanceOf::<T>::zero(), Error::<T>::InvalidLiquidityWithdrawal);

			// Retrieve the lending pool and update the indexes
			let asset_pool = AssetPool::<T>::from(asset);
			let mut pool = LendingPoolStorage::<T>::get(&asset_pool)
				.ok_or_else(|| DispatchError::from(Error::<T>::LendingPoolDoesNotExist))?;
			pool.update_indexes()?;

			// Retrieve the loan and calculate the repayable amount
			let mut loan = Borrows::<T>::get((who, asset, collateral_asset))
				.ok_or(Error::<T>::LoanDoesNotExists)?;
			let repayable_balance = pool.repayable_amount(&loan)?;

			// Determine the payment amount and whether it's a full payment
			let (pay, is_full_payment) = if balance <= repayable_balance {
				(balance, false)
			} else {
				(repayable_balance, true)
			};

			// Calculate the ratio of the repayment to the total repayable amount using u128
			let repay_ratio_numerator = pay.saturated_into::<u128>();
			let repay_ratio_denominator = repayable_balance.saturated_into::<u128>();

			// Ensure the denominator is not zero
			ensure!(repay_ratio_denominator > 0, Error::<T>::OverflowError);

			// Calculate the repay ratio as a FixedU128
			let repay_ratio =
				FixedU128::checked_from_rational(repay_ratio_numerator, repay_ratio_denominator)
					.ok_or(Error::<T>::OverflowError)?;

			// Update the loan's borrowed balance
			let borrowed_balance_u128 = loan.borrowed_balance.saturated_into::<u128>();
			let borrowed_balance_reduction_u128 = repay_ratio
				.checked_mul(&FixedU128::from(borrowed_balance_u128))
				.ok_or(Error::<T>::OverflowError)?
				.into_inner() / FixedU128::accuracy();

			// Update the pool balances
			let borrowed_balance_reduction =
				borrowed_balance_reduction_u128.saturated_into::<AssetBalanceOf<T>>();
			pool.move_asset_on_repay(pay, borrowed_balance_reduction)?;

			// Transfer the repayment amount to the market
			T::Fungibles::transfer(
				asset.clone(),
				who,
				&Self::account_id(),
				pay,
				Preservation::Preserve,
			)?;

			let new_borrowed_balance_u128 =
				borrowed_balance_u128.saturating_sub(borrowed_balance_reduction_u128);

			loan.borrowed_balance = new_borrowed_balance_u128.saturated_into::<AssetBalanceOf<T>>();

			let release_collateral_amount =
				Self::get_release_collateral_amount(repay_ratio, loan.collateral_balance)?;

			loan.collateral_balance = loan
				.collateral_balance
				.checked_sub(&release_collateral_amount)
				.ok_or(Error::<T>::OverflowError)?;

			// Update the loan or remove it if fully repaid
			if is_full_payment {
				Borrows::<T>::remove((who, asset, collateral_asset));
				// Release all the collateral
				T::Fungibles::transfer(
					collateral_asset.clone(),
					&Self::account_id(),
					who,
					loan.collateral_balance,
					Preservation::Preserve,
				)?;
			} else {
				// Update the loan
				Borrows::<T>::insert((who, asset, collateral_asset), loan);
				// Release partial collateral
				T::Fungibles::transfer(
					collateral_asset.clone(),
					&Self::account_id(),
					who,
					release_collateral_amount,
					Preservation::Expendable,
				)?;
			}

			// Update the storage with the new pool state
			LendingPoolStorage::<T>::insert(&asset_pool, pool);

			Ok(())
		}

		/// This method de-activates an existing lending pool
		pub fn do_deactivate_lending_pool(asset: AssetIdOf<T>) -> DispatchResult {
			// let's check if our pool does exist before de-activating it
			let asset_pool = AssetPool::<T>::from(asset);
			let mut pool = LendingPoolStorage::<T>::get(&asset_pool)
				.ok_or_else(|| DispatchError::from(Error::<T>::LendingPoolDoesNotExist))?;

			// let's check if our pool is actually already non-active
			ensure!(pool.is_active() == true, Error::<T>::LendingPoolAlreadyDeactivated);

			// ok now we can de-activate it
			pool.activated = false;
			LendingPoolStorage::set(&asset_pool, Some(pool.clone()));
			Ok(())
		}

		/// This method returns the palled account id
		///
		/// This actually does computation. If you need to keep using it,
		/// then make sure to cache the value and only call this once.
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		/// Calculates the new_mint amount as follows,
		/// 	let interest_on_old_deposit = old_balance * (current_supply_index - last_supply_index)
		/// 	let total_new_mint = interest_on_old_deposit + new balance
		/// Mint total_new_mint LP tokens into supplier's account
		/// Update supplier data
		pub fn update_and_mint(
			who: &T::AccountId,
			asset: AssetIdOf<T>,
			lp_id: AssetIdOf<T>,
			scaled_balance: AssetBalanceOf<T>,
			current_supply_index: Rate,
		) -> DispatchResult {
			let supply_index = SupplyIndexStorage::<T>::get((who, asset));
			let old_balance = T::Fungibles::balance(lp_id, who);

			let interest_on_old_deposit = (current_supply_index
				.checked_sub(&supply_index.supply_index)
				.ok_or(Error::<T>::OverflowError)?)
			.checked_mul(&FixedU128::from(old_balance.saturated_into::<u128>()))
			.ok_or(Error::<T>::OverflowError)?;

			let total_new_mint = scaled_balance
				.checked_add(&interest_on_old_deposit.into_inner().saturated_into())
				.ok_or(Error::<T>::OverflowError)?;
			let updated_supply_index = SupplyIndex {
				supply_index: current_supply_index,
				last_accrued_interest_at: Self::now_in_seconds(),
			};
			SupplyIndexStorage::<T>::insert((who, asset), updated_supply_index);

			T::Fungibles::mint_into(lp_id, who, scaled_balance)?;
			Self::deposit_event(Event::LPTokenMinted {
				who: who.clone(),
				asset: lp_id,
				balance: total_new_mint,
			});
			Ok(())
		}

		/// Returns the block's timestamp in seconds as u64
		fn now_in_seconds() -> u64 {
			core::time::Duration::from_millis(T::Time::now().saturated_into::<u64>())
				.as_secs()
				.saturated_into::<u64>()
		}

		pub fn compute_user_ltv(account: &T::AccountId) -> (FixedU128, FixedU128, FixedU128) {
			let mut total_borrowed_usdt: u128 = 0;
			let mut total_collateral_usdt: u128 = 0;
			let mut min_collateral_factor = Permill::from_percent(100);
			let mut min_liquidation_threshold = Permill::from_percent(100);

			// Iterate over all borrows for the account
			for ((borrower, borrowed_asset, collateral_asset), loan) in Borrows::<T>::iter() {
				if borrower != *account {
					continue;
				}
				// Get the lending pool for the borrowed asset
				let asset_pool = AssetPool::<T>::from(borrowed_asset);
				let mut pool = match LendingPoolStorage::<T>::get(&asset_pool) {
					Some(p) => p,
					None => continue, // Skip if no pool found
				};

				// Update pool indexes
				if pool.update_indexes().is_err() {
					continue;
				}

				// Compute the repayable amount (current borrowed balance with interest)
				let repayable_amount = match pool.repayable_amount(&loan) {
					Ok(amount) => amount,
					Err(_) => continue,
				};

				// Convert repayable_amount to USDT equivalent
				let borrowed_usdt = match Self::get_equivalent_asset_amount(
					1u32.into(), // Assuming 1 is the asset ID for USDT
					borrowed_asset,
					repayable_amount,
				) {
					Ok(amount) => amount,
					Err(_) => continue,
				};

				total_borrowed_usdt =
					total_borrowed_usdt.saturating_add(borrowed_usdt.saturated_into::<u128>());

				// Convert collateral to USDT equivalent
				let collateral_usdt = match Self::get_equivalent_asset_amount(
					1u32.into(), // Assuming 1 is the asset ID for USDT
					collateral_asset,
					loan.collateral_balance,
				) {
					Ok(amount) => amount,
					Err(_) => continue,
				};

				total_collateral_usdt =
					total_collateral_usdt.saturating_add(collateral_usdt.saturated_into::<u128>());

				// Update minimum collateral factors
				if pool.collateral_factor < min_collateral_factor {
					min_collateral_factor = pool.collateral_factor;
				}

				if pool.liquidation_threshold < min_liquidation_threshold {
					min_liquidation_threshold = pool.liquidation_threshold;
				}
			}

			// Calculate current LTV
			let current_ltv = if !total_collateral_usdt.is_zero() {
				FixedU128::checked_from_rational(total_borrowed_usdt, total_collateral_usdt)
					.unwrap_or_else(|| FixedU128::zero())
			} else {
				FixedU128::zero()
			};

			// Convert ratios to FixedU128
			let sale_ltv: FixedU128 = min_collateral_factor.into();
			let liquidation_ltv: FixedU128 = min_liquidation_threshold.into();

			(current_ltv, sale_ltv, liquidation_ltv)
		}

		/// Returns the amount of asset equivalent to the collateral
		/// checks if price of collateral asset available in terms of asset then
		/// return `price * collateral_balance `
		/// else
		/// check if price of asset available in terms of collateral asset then
		/// return `collateral_balance / price`
		/// else fallback to a common base asset
		/// get the prices of both assets in terms of asset 1 (USDT)  and
		/// return `collateral_asset_price * collateral_balance / asset_price`
		/// else
		/// return error `AssetPriceNotSet`
		pub fn get_equivalent_asset_amount(
			asset: AssetIdOf<T>,
			collateral_asset: AssetIdOf<T>,
			collateral_balance: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let amount = if let Some(p) = AssetPrices::<T>::get((collateral_asset, asset)) {
				p.checked_mul(&FixedU128::from_inner(collateral_balance.saturated_into()))
					.ok_or(Error::<T>::OverflowError)?
			} else if let Some(p) = AssetPrices::<T>::get((asset, collateral_asset)) {
				FixedU128::from_inner(collateral_balance.saturated_into())
					.checked_div(&p)
					.ok_or(Error::<T>::OverflowError)?
			} else {
				let asset_price =
					AssetPrices::<T>::get((asset, 1)).ok_or(Error::<T>::AssetPriceNotSet)?;
				let collateral_price = AssetPrices::<T>::get((collateral_asset, 1))
					.ok_or(Error::<T>::AssetPriceNotSet)?;

				collateral_price
					.checked_div(&asset_price)
					.ok_or(Error::<T>::OverflowError)?
					.checked_mul(&FixedU128::from_inner(collateral_balance.saturated_into()))
					.ok_or(Error::<T>::OverflowError)?
			}
			.into_inner()
			.saturated_into();
			Ok(amount)
		}

		/// Returns the amount of collateral asset to be released on partial repayment
		/// Returns release_amount = pay / repayable_balance * collateral_balance
		fn get_release_collateral_amount(
			repay_ratio: FixedU128,
			collateral_balance: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let collateral_balance_u128 = collateral_balance.saturated_into::<u128>();

			let release_collateral_amount_u128 = repay_ratio
				.checked_mul(&FixedU128::from(collateral_balance_u128))
				.ok_or(Error::<T>::OverflowError)?
				.into_inner() / FixedU128::accuracy();

			let release_collateral_amount =
				release_collateral_amount_u128.saturated_into::<AssetBalanceOf<T>>();

			Ok(release_collateral_amount)
		}

		/// Fetches the balance of a given asset for an account.
		///
		/// # Arguments
		///
		/// * `account` - The account for which to fetch the balance.
		/// * `asset` - The `AssetId` of the asset to fetch.
		///
		/// # Returns
		///
		/// * `Result<AssetBalanceOf<T>, Error<T>>` - The balance of the asset or an error.
		pub fn get_asset_balance(
			account: &T::AccountId,
			asset: AssetIdOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let user_balance = T::Fungibles::balance(asset.clone(), account);
			Ok(user_balance)
		}

		/// Retrieves the price of a given asset relative to a base asset.
		///
		/// The function checks if a direct price is available between `asset` and `base_asset`.
		/// If not, calculates the price using an intermediary (e.g., USDT).
		///
		/// # Arguments
		///
		/// * `asset` - The `AssetId` of the asset.
		/// * `base_asset` - The `AssetId` of the base asset.
		///
		/// # Returns
		///
		/// * `Result<FixedU128, Error<T>>` - The price of the asset or an error if not found or overflow occurs.
		pub fn get_asset_price(
			asset: AssetIdOf<T>,
			base_asset: AssetIdOf<T>,
		) -> Result<FixedU128, Error<T>> {
			let price = if let Some(p) = AssetPrices::<T>::get((asset, base_asset)) {
				let base_asset_decimals = pallet_assets::Pallet::<T>::decimals(base_asset);
				let decimals_divisor = 10u128
					.checked_pow(base_asset_decimals as u32)
					.ok_or(Error::<T>::OverflowError)?;
				p.checked_div(&FixedU128::from(decimals_divisor))
					.ok_or(Error::<T>::DivisionByZero)?
			} else {
				// Calculate asset price through a common base asset (USDT)
				let asset_usdt_price = AssetPrices::<T>::get((asset, 1))
					.ok_or(Error::<T>::AssetPriceNotSet)?;
				let base_asset_usdt_price = AssetPrices::<T>::get((base_asset, 1))
					.ok_or(Error::<T>::AssetPriceNotSet)?;

				asset_usdt_price
					.checked_div(&base_asset_usdt_price)
					.ok_or(Error::<T>::DivisionByZero)?
			};

			Ok(price)
		}

		/// Retrieves metadata for a given asset.
		///
		/// # Arguments
		///
		/// * `asset` - The `AssetId` of the asset.
		///
		/// # Returns
		///
		/// A tuple containing:
		/// - `Vec<u8>`: Asset name.
		/// - `u8`: Asset decimals.
		/// - `Vec<u8>`: Asset symbol.
		pub fn get_metadata(asset: AssetIdOf<T>) -> (Vec<u8>, u8, Vec<u8>) {
			let asset_name = <pallet_assets::Pallet<T> as MetadataInspect<_>>::name(asset);
			let asset_decimals = <pallet_assets::Pallet<T> as MetadataInspect<_>>::decimals(asset);
			let asset_symbol = <pallet_assets::Pallet<T> as MetadataInspect<_>>::symbol(asset);
			(asset_name, asset_decimals, asset_symbol)
		}

		/// Retrieves lending pools and their aggregated totals.
		///
		/// This function collects all the lending pools matching the given `asset` filter and aggregates their total supply and borrow amounts.
		/// 
		/// # Arguments
		///
		/// * `asset` - An optional filter for a specific `AssetId`. If `None`, includes all pools.
		/// * `account` - An optional reference to the account for which to fetch the balance.
		///
		/// # Returns
		///
		/// A tuple of:
		/// * `Vec<LendingPoolInfo>` - A vector containing details of each lending pool.
		/// * `AggregatedTotals` - A struct containing the total supply and borrow amounts.
		pub fn get_lending_pools(asset: Option<AssetIdOf<T>>, account: Option<&T::AccountId>) -> (Vec<LendingPoolInfo>, AggregatedTotals) {
			let mut total_supply: u128 = 0;
			let mut total_borrow: u128 = 0;

			// Collect all lending pools and calculate aggregated totals in a single iteration
			let pools: Vec<LendingPoolInfo> = LendingPoolStorage::<T>::iter()
				.filter(|(_, pool)| {
					match asset {
						Some(asset_id) => pool.lend_token_id == asset_id,
						None => true, // Include all pools if `asset` is `None`
					}
				})
				.map(|(_, pool)| {
					// Retrieve metadata for the pool's asset
					let (asset_name, asset_decimals, asset_symbol) = Self::get_metadata(pool.lend_token_id);
					let asset_icon = "<url>/dot.svg".as_bytes().to_vec(); // Placeholder for asset icon
					
					let user_asset_balance = match account {
						Some(account) => Self::get_asset_balance(&account, pool.lend_token_id).ok().map(|balance| balance.saturated_into::<u128>()),
						None => None,
					};

					// Convert reserve balance and borrowed balance to equivalent asset amount in USDT
					let equivalent_asset_supply_amount = Self::get_equivalent_asset_amount(
						1, // USDT
						pool.lend_token_id,
						pool.reserve_balance,
					).unwrap_or_default();
					
					let equivalent_asset_borrow_amount = Self::get_equivalent_asset_amount(
						1, // USDT
						pool.lend_token_id,
						pool.borrowed_balance,
					).unwrap_or_default();

					// Accumulate totals
					total_supply = total_supply.saturating_add(equivalent_asset_supply_amount.saturated_into::<u128>());
					total_borrow = total_borrow.saturating_add(equivalent_asset_borrow_amount.saturated_into::<u128>());

					LendingPoolInfo {
						id: pool.id,
						asset_id: pool.lend_token_id,
						asset: asset_name,
						asset_decimals: asset_decimals as u32,
						asset_symbol: asset_symbol,
						asset_icon: asset_icon,
						collateral_q: pool.collateral_factor.deconstruct().into(),
						utilization: pool.utilisation_ratio().unwrap_or_default().into(),
						borrow_apy: pool.borrow_interest_rate().unwrap_or_default().into(),
						borrow_apy_s: FixedU128::zero(), // Placeholder value
						supply_apy: pool.supply_interest_rate().unwrap_or_default().into(),
						supply_apy_s: FixedU128::zero(), // Placeholder value
						is_activated: pool.activated,
						user_asset_balance,
					}
				})
				.collect();

			let aggregated_totals = AggregatedTotals {
				total_supply,
				total_borrow,
			};

			(pools, aggregated_totals)
		}

		/// Retrieves supplied assets for a given account and calculates total deposits.
		///
		/// This function iterates over all lending pools and collects the supplies for the given `account`.
		///
		/// # Arguments
		///
		/// * `account` - The account for which to retrieve the supplies.
		///
		/// # Returns
		///
		/// A tuple of:
		/// * `Vec<SuppliedAsset>` - A vector of all assets supplied by the account.
		/// * `TotalDeposit` - The total amount of assets deposited.
		pub fn get_asset_wise_supplies(
			account: &T::AccountId,
		) -> (Vec<SuppliedAsset>, TotalDeposit) {
			let mut total_supply: u128 = 0;

			// Iterate over all lending pools to gather the supplies for the given account
			let supplied_assets: Vec<SuppliedAsset> = LendingPoolStorage::<T>::iter()
				.filter_map(|(_, mut pool)| {
					// Get the account's LP token balance for this pool
					let lp_balance = Self::get_asset_balance(&account, pool.id).ok()?;
					if lp_balance.is_zero() {
						return None;
					}

					// Update pool indexes
					if pool.update_indexes().is_err() {
						return None;
					}

					// Calculate the supplied amount by multiplying LP balance with supply index
					let supplied_amount = pool.accrued_deposit(lp_balance).ok()?;

					let asset_balance = Self::get_asset_balance(&account, pool.lend_token_id)
						.ok()?
						.saturated_into::<u128>();

					// Retrieve metadata for the pool's asset
					let (asset_name, asset_decimals, asset_symbol) = Self::get_metadata(pool.lend_token_id);
					let asset_icon = "<url>/dot.svg".as_bytes().to_vec(); // Placeholder for asset icon

					// Calculate the equivalent supplied amount
					let equivalent_supplied_amount = Self::get_equivalent_asset_amount(
						1, // USDT
						pool.lend_token_id,
						supplied_amount,
					).unwrap_or_default();

					// Calculate the current APY for this pool
					let apy = pool.supply_interest_rate().unwrap_or_default();

					// Accumulate total supply
					total_supply = total_supply.saturating_add(equivalent_supplied_amount.saturated_into::<u128>());

					// Create and return a `SuppliedAsset`
					Some(SuppliedAsset {
						asset_info: AssetInfo {
							asset_id: pool.lend_token_id,
							asset_name,
							asset_symbol,
							decimals: asset_decimals,
							asset_icon,
							balance: asset_balance,
						},
						apy,
						supplied: supplied_amount.saturated_into::<u128>(),
					})
				})
				.collect();

			(supplied_assets, total_supply)
		}

		/// Retrieves borrowed and collateral assets for a given account and calculates totals.
		///
		/// This function iterates over all active borrows for the account and collects the borrowed and collateral assets.
		///
		/// # Arguments
		///
		/// * `account` - The account for which to retrieve the borrowed and collateral assets.
		///
		/// # Returns
		///
		/// A tuple of:
		/// * `Vec<BorrowedAsset>` - A list of assets borrowed by the account.
		/// * `Vec<CollateralAsset>` - A list of collateral assets associated with the account's borrows.
		/// * `TotalBorrow` - The total amount borrowed.
		/// * `TotalCollateral` - The total collateral provided.
		pub fn get_asset_wise_borrows_collaterals(
			account: &T::AccountId,
		) -> (Vec<BorrowedAsset>, Vec<CollateralAsset>, TotalBorrow, TotalCollateral) {
			let mut total_borrow: u128 = 0;
			let mut total_collateral: u128 = 0;

			let mut borrowed_assets: Vec<BorrowedAsset> = Vec::new();
			let mut collateral_assets: Vec<CollateralAsset> = Vec::new();

			// Iterate over all borrows to collect information for the given account
			for ((borrower, borrowed_asset, collateral_asset), loan) in Borrows::<T>::iter() {
				// Process only entries for the given account
				if borrower != *account {
					continue;
				}

				// Get the lending pool for the borrowed asset
				let asset_pool = AssetPool::<T>::from(borrowed_asset);
				let mut pool = match LendingPoolStorage::<T>::get(&asset_pool) {
					Some(p) => p,
					None => continue, // Skip if no pool is found
				};

				// Update pool indexes
				if pool.update_indexes().is_err() {
					continue;
				}

				// Compute the repayable amount, considering accrued interest
				let borrowed_amount = match pool.repayable_amount(&loan) {
					Ok(amount) => amount,
					Err(_) => continue,
				};

				// Handle borrowed assets
				let borrow_balance = Self::get_asset_balance(&account, borrowed_asset)
					.ok()
					.map(|amount| amount.saturated_into::<u128>())
					.unwrap_or_default();

				// Retrieve asset metadata for the borrowed asset
				let (borrow_asset_name, borrow_asset_decimals, borrow_asset_symbol) = Self::get_metadata(borrowed_asset);
				let borrow_asset_icon = "<url>/dot.svg".as_bytes().to_vec(); // Placeholder for asset icon

				// Calculate equivalent borrowed amount in USDT
				let equivalent_borrowed_amount = Self::get_equivalent_asset_amount(
					1, // USDT
					borrowed_asset,
					borrowed_amount,
				).unwrap_or_default();

				// Accumulate total borrowed amount
				total_borrow = total_borrow.saturating_add(equivalent_borrowed_amount.saturated_into::<u128>());

				// Create and store a `BorrowedAsset`
				borrowed_assets.push(BorrowedAsset {
					asset_info: AssetInfo {
						asset_id: borrowed_asset,
						asset_symbol: borrow_asset_symbol,
						asset_name: borrow_asset_name,
						decimals: borrow_asset_decimals,
						asset_icon: borrow_asset_icon,
						balance: borrow_balance,
					},
					apy: pool.borrow_interest_rate().unwrap_or_default(),
					borrowed: borrowed_amount.saturated_into::<u128>(),
				});

				// Handle collateral assets
				let collateral_balance = loan.collateral_balance;

				// Retrieve asset metadata for the collateral asset
				let (collateral_asset_name, collateral_asset_decimals, collateral_asset_symbol) = Self::get_metadata(collateral_asset);
				let collateral_asset_icon = "<url>/dot.svg".as_bytes().to_vec(); // Placeholder for asset icon

				// Calculate equivalent collateral amount in USDT
				let equivalent_collateral_amount = Self::get_equivalent_asset_amount(
					1, // USDT
					collateral_asset,
					collateral_balance,
				).unwrap_or_default();

				// Accumulate total collateral amount
				total_collateral = total_collateral.saturating_add(equivalent_collateral_amount.saturated_into::<u128>());

				// Create and store a `CollateralAsset`
				collateral_assets.push(CollateralAsset {
					asset_info: AssetInfo {
						asset_id: collateral_asset,
						asset_symbol: collateral_asset_symbol,
						asset_name: collateral_asset_name,
						decimals: collateral_asset_decimals,
						asset_icon: collateral_asset_icon,
						balance: collateral_balance.saturated_into::<u128>(),
					},
				});
			}

			// Return the list of borrowed assets, collateral assets, total borrow, and total collateral
			(borrowed_assets, collateral_assets, total_borrow, total_collateral)
		}
	}
}
