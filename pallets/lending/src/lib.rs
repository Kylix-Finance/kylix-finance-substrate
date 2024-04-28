#![cfg_attr(not(feature = "std"), no_std)]

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
	sp_runtime::{FixedU128, Permill, SaturatedConversion},
	traits::{fungible, fungibles},
};
pub use frame_support::{
	sp_runtime,
	sp_runtime::traits::{
		AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero,
	},
	traits::{
		fungibles::{Create, Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		Time as MomentTime,
	},
	DefaultNoBound, PalletId,
};
pub use frame_system::pallet_prelude::*;
pub use pallet::*;
pub use sp_runtime::FixedPointNumber;

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
pub const SECONDS_PER_YEAR: u64 = 365u64 * 24 * 60 * 60;

mod borrow_repay;
use borrow_repay::UserBorrow;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
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

	// let's hardcore a default interest rate model (the same AAVE has)
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
	pub struct InterestRateModel {
		base_rate: Rate,
		slope1: Rate,
		slope2: Rate,
		kink: Rate,
	}
	impl InterestRateModel {
		/// Default interest rate model.
		/// TODO: change to a dynamic model defined by the pool creator
		pub fn hardcoded_default_interest() -> Self {
			InterestRateModel {
				base_rate: Rate::saturating_from_rational(2, 100),
				slope1: Rate::saturating_from_rational(4, 100),
				slope2: Rate::saturating_from_rational(75, 100),
				kink: Rate::saturating_from_rational(80, 100),
			}
		}

		pub fn base_rate(&self) -> Rate {
			self.base_rate
		}
		pub fn slope1(&self) -> Rate {
			self.slope1
		}
		pub fn slope2(&self) -> Rate {
			self.slope2
		}
		pub fn kink(&self) -> Rate {
			self.kink
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

				interest_model: InterestRateModel::hardcoded_default_interest(),
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

		///
		/// The BORROW interest rate model calculated as
		///
		/// if (utilisation_ratio <= kink)
		/// 	base_rate + (utilization_ratio/kink) * slope1
		///
		/// if (utilisation_ratio > kink)
		/// 	base_rate + slope1 + ((utilisation_ratio - kink)/(1 - kink)) * slope2
		pub fn borrow_interest_rate(&self) -> Result<Rate, Error<T>> {
			if self.borrowed_balance.is_zero() || self.reserve_balance.is_zero() {
				return Ok(Rate::zero());
			}

			let utilisation_ratio = self.utilisation_ratio()?;

			let base = self.interest_model.base_rate();
			let slope1 = self.interest_model.slope1();
			let slope2 = self.interest_model.slope2();
			let kink = self.interest_model.kink();

			let utilisation_ratio: Rate = utilisation_ratio.into();

			if utilisation_ratio <= kink {
				let res = utilisation_ratio
					.checked_div(&kink)
					.ok_or(Error::<T>::OverflowError)?
					.checked_mul(&slope1)
					.ok_or(Error::<T>::OverflowError)?;

				let borrow_rate = base.checked_add(&res).ok_or(Error::<T>::OverflowError)?;

				return Ok(borrow_rate);
			}

			// utilisation_ratio > kink

			let numerator =
				utilisation_ratio.checked_sub(&kink).ok_or(Error::<T>::OverflowError)?;

			let denominator = Rate::saturating_from_rational(100, 100) // 100%_
				.checked_sub(&kink)
				.ok_or(Error::<T>::OverflowError)?;

			let partial = slope2
				.checked_mul(&numerator)
				.ok_or(Error::<T>::OverflowError)?
				.checked_div(&denominator)
				.ok_or(Error::<T>::OverflowError)?;

			let ut = base
				.checked_add(&slope1)
				.ok_or(Error::<T>::OverflowError)?
				.checked_mul(&partial)
				.ok_or(Error::<T>::OverflowError)?;

			Ok(ut)
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

		/// Calculate compounded interest
		/// Borrow interest compounds every second. This is achieved by an approximation of binomial
		/// expansion to the third term.
		/// BinomalExpansion:(1+x)^n = 1+ nx + (n/2)(n−1)*x^2 + (n/6)(n−1)(n−2)x^3 +...
		/// Where n = t, number of periods and x = r, rate per second
		/// Interest:(1+r) t ≈1 + rt + t/2 * (t−1) * r^2 + (t/6) * (t−1) * (t−2) * r^3
		fn calculate_compunded_interest(&self) -> Result<Rate, Error<T>> {
			let rate = self
				.borrow_interest_rate()?
				.checked_div(&(SECONDS_PER_YEAR as u128).into())
				.ok_or(Error::<T>::OverflowError)?;
			let t = Pallet::<T>::now_in_seconds()
				.checked_sub(self.last_accrued_interest_at)
				.ok_or(Error::<T>::OverflowError)?;
			let t_minus_one = t.checked_sub(1u64).ok_or(Error::<T>::OverflowError)?;
			let t_minus_two = t.checked_sub(2u64).ok_or(Error::<T>::OverflowError)?;
			let rate_square = rate.checked_mul(&rate).ok_or(Error::<T>::OverflowError)?;
			let rate_cube = rate_square.checked_mul(&rate).ok_or(Error::<T>::OverflowError)?;

			let first_term =
				rate.checked_mul(&(t as u128).into()).ok_or(Error::<T>::OverflowError)?;

			let second_term = FixedU128::from(t as u128)
				.checked_mul(&(t_minus_one as u128).into())
				.ok_or(Error::<T>::OverflowError)?
				.checked_mul(&rate_square)
				.ok_or(Error::<T>::OverflowError)?
				.checked_div(&(2u128).into())
				.ok_or(Error::<T>::OverflowError)?;

			let third_term = FixedU128::from(t as u128)
				.checked_mul(&(t_minus_one as u128).into())
				.ok_or(Error::<T>::OverflowError)?
				.checked_mul(&(t_minus_two as u128).into())
				.ok_or(Error::<T>::OverflowError)?
				.checked_mul(&rate_cube)
				.ok_or(Error::<T>::OverflowError)?
				.checked_div(&(6u128).into())
				.ok_or(Error::<T>::OverflowError)?;
			let interest = FixedU128::one()
				.checked_add(&first_term)
				.ok_or(Error::<T>::OverflowError)?
				.checked_add(&second_term)
				.ok_or(Error::<T>::OverflowError)?
				.checked_add(&third_term)
				.ok_or(Error::<T>::OverflowError)?;

			Ok(interest)
		}

		fn update_supply_index(&mut self) -> Result<(), Error<T>> {
			let incr = self.calculate_linear_interest()?;
			let new_index =
				self.supply_index.checked_mul(&incr).ok_or(Error::<T>::OverflowError)?;
			self.supply_index = new_index;
			Ok(())
		}

		fn udpate_borrow_index(&mut self) -> Result<(), Error<T>> {
			let incr = self.calculate_compunded_interest()?;
			let new_index =
				self.borrow_index.checked_mul(&incr).ok_or(Error::<T>::OverflowError)?;
			self.borrow_index = new_index;
			Ok(())
		}

		fn update_indexes(&mut self) -> Result<(), Error<T>> {
			if self.last_accrued_interest_at < Pallet::<T>::now_in_seconds() {
				self.update_supply_index()?;
				self.udpate_borrow_index()?;
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
		pub fn move_asset_on_repay(&mut self, balance: AssetBalanceOf<T>) -> Result<(), Error<T>> {
			self.borrowed_balance =
				self.borrowed_balance.checked_sub(&balance).ok_or(Error::<T>::OverflowError)?;
			self.reserve_balance =
				self.reserve_balance.checked_add(&balance).ok_or(Error::<T>::OverflowError)?;
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

		/// Calculate the repayable amount including borrow interests
		/// _amount = borrow_balance * borrow_index
		pub fn repayable_amount(
			&self,
			borrow_balance: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let r = FixedU128::from_inner(borrow_balance.saturated_into())
				.checked_mul(&self.borrow_index)
				.ok_or(Error::<T>::OverflowError)?
				.into_inner()
				.saturated_into();
			Ok(r)
		}
	}

	/// Kylix runtime storage items
	///
	/// Lending pools defined for the assets
	///
	/// StorageMap AssetPool { AssetId } => LendingPool { PoolId, Balance }
	#[pallet::storage]
	#[pallet::getter(fn reserve_pools)]
	pub type LendingPoolStorage<T> =
		StorageMap<_, Blake2_128Concat, AssetPool<T>, LendingPool<T>, ValueQuery>;

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
	/// (AccountId, corrowed_asset_id, collateral_asset_id) => UserBorrow details
	#[pallet::storage]
	pub type Borrows<T: Config> =
		StorageMap<_, Blake2_128Concat, (AccountOf<T>, AssetIdOf<T>, AssetIdOf<T>), UserBorrow<T>>;

	/// The storage to hold prices of assets w.r.t. other other assets
	/// This is the dummy storage, ideally this functionality would be implemented in a dedicatd
	/// pallet sotres (asset_id1, asset_id2) => FixedU128
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
		NotEnoughElegibleLiquidityToWithdraw,
		/// Lending Pool is empty
		LendingPoolIsEmpty,
		/// The classic Overflow Error
		OverflowError,
		/// The ID already exists
		IdAlreadyExists,
		/// The user has not enough collateral assets
		NotEnoughCollateral,
		/// The Loan being repayed does not exists
		LoanDoesNotExists,
		/// Price of the asset can not be zero
		InvalidAssetPrice,
		/// The price of the asset is not avaialble
		AssetPriceNotSet,
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
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflows or
		///   underflows
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
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflows or
		///  underflows
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
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflows or
		/// underflows
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
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflows or
		/// underflows
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
		/// * If adding liquidity to the pool fails for any reason due to arithmetic overflows or
		/// underflows
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

			// Second, let's check the if user has enough liquidity
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

			LendingPoolStorage::<T>::insert(asset_pool, &lending_pool);

			// let's transfers the tokens (asset) from the users account into pallet account
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
			ensure!(
				LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolDoesNotExist
			);

			// let's check if our pool is actually already active and balance > 0
			let pool = LendingPoolStorage::<T>::get(asset_pool.clone());
			ensure!(pool.is_active() == false, Error::<T>::LendingPoolAlreadyActivated);
			ensure!(!pool.is_empty(), Error::<T>::LendingPoolIsEmpty);

			// ok now we can activate it
			LendingPoolStorage::<T>::mutate(asset_pool, |v| v.activated = true);
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

			// Second, let's check the if user has enough liquidity tp supply
			let user_balance = T::Fungibles::balance(asset.clone(), who);
			ensure!(user_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			// let's check if our pool does exist
			let asset_pool = AssetPool::<T>::from(asset);
			ensure!(
				LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolDoesNotExist
			);

			let mut pool = LendingPoolStorage::<T>::get(&asset_pool);

			// let's ensure that the lending pool is active
			ensure!(pool.is_active() == true, Error::<T>::LendingPoolNotActive);

			// Update pool supply index
			pool.update_indexes()?;
			pool.reserve_balance =
				pool.reserve_balance.checked_add(&balance).ok_or(Error::<T>::OverflowError)?;

			// let's transfers the tokens (asset) from the users account into pallet account
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
			LendingPoolStorage::<T>::set(&asset_pool, pool);

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
			ensure!(
				LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolDoesNotExist
			);

			let mut pool = LendingPoolStorage::<T>::get(asset_pool.clone());

			// let's check the if the pool has enough liquidity
			ensure!(pool.reserve_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			// Update pool's indexes
			pool.update_indexes()?;

			// let's check if the user is actually elegible to withdraw!
			let scaled_lp_tokens = T::Fungibles::balance(pool.id.clone(), &who);
			let eligible_lp_tokens = pool.accrued_deposit(scaled_lp_tokens)?;
			ensure!(
				eligible_lp_tokens >= balance,
				Error::<T>::NotEnoughElegibleLiquidityToWithdraw
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
			LendingPoolStorage::<T>::set(&asset_pool, pool);

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
			ensure!(
				LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolDoesNotExist
			);
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
			let mut pool = LendingPoolStorage::<T>::get(asset_pool.clone());
			ensure!(pool.is_active() == true, Error::<T>::LendingPoolNotActive);

			// let's check the if the pool has enough liquidity
			ensure!(pool.reserve_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			// Update pool's indexex
			pool.update_indexes()?;

			// check sufficiency of collateral asset
			// get collateral asset value in terms of borrow-asset
			let equivalent_asset_balace = Self::get_equivalent_asset_amount(
				who,
				asset,
				collateral_asset,
				collateral_balance,
			)?;
			// get elligible borrow quantity based on reserve_factor
			let eligible_asset_amount = pool.max_borrow_amount(equivalent_asset_balace)?;
			// error if borrow is more than eligibility
			ensure!(eligible_asset_amount >= balance, Error::<T>::NotEnoughCollateral);

			// Save sacled balance as per current borrow_index
			let scaled_balance = pool.scaled_borrow_balance(balance)?;

			let borrow: UserBorrow<T> = UserBorrow {
				borrowed_asset: asset,
				borrowed_balance: scaled_balance,
				collateral_asset,
				collateral_balance,
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

			LendingPoolStorage::<T>::set(&asset_pool, pool);

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

			// let's check if our pool does exist
			let asset_pool = AssetPool::<T>::from(asset);
			ensure!(
				LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolDoesNotExist
			);

			// get the lending pool and update the indexes
			let mut pool = LendingPoolStorage::<T>::get(&asset_pool);
			pool.update_indexes()?;

			// get the repay amount and check if loan exists
			let mut loan = Borrows::<T>::get((who, asset, collateral_asset))
				.ok_or(Error::<T>::LoanDoesNotExists)?;
			let repayable_balance = pool.repayable_amount(loan.borrowed_balance)?;

			// take max upto repayable amount
			let (pay, is_full_payment) = if balance <= repayable_balance {
				(balance, false)
			} else {
				(repayable_balance, true)
			};

			// Update pool: transfer asset from reserved_balance to borrowed_balance
			pool.move_asset_on_repay(pay)?;

			// transfer repay amount to the market
			T::Fungibles::transfer(
				asset.clone(),
				who,
				&Self::account_id(),
				pay,
				Preservation::Preserve,
			)?;

			if is_full_payment {
				// clear the borrow
				Borrows::<T>::remove((who, asset, collateral_asset));
				// release all the collateral
				T::Fungibles::transfer(
					collateral_asset.clone(),
					&Self::account_id(),
					who,
					loan.collateral_balance,
					Preservation::Preserve,
				)?;
			} else {
				// get the amount of collateral to release
				// pay / repayable_balance * collateral_balance
				let release_collateral_amount: AssetBalanceOf<T> =
					Self::get_release_collateral_amount(
						pay,
						repayable_balance,
						loan.collateral_balance,
					)?;
				// repay the borrow
				let scaled_pay = pool.scaled_borrow_balance(pay)?;
				loan.repay_partial(scaled_pay, release_collateral_amount)?;
				Borrows::<T>::set((who, asset, collateral_asset), Some(loan));
				// release partial collateral
				T::Fungibles::transfer(
					collateral_asset.clone(),
					&Self::account_id(),
					who,
					release_collateral_amount,
					Preservation::Preserve,
				)?;
			}

			// emit event

			Ok(())
		}

		/// This method de-activates an existing lending pool
		pub fn do_deactivate_lending_pool(asset: AssetIdOf<T>) -> DispatchResult {
			// let's check if our pool does exist before de-activating it
			let asset_pool = AssetPool::<T>::from(asset);
			ensure!(
				LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolDoesNotExist
			);

			// let's check if our pool is actually already non-active
			let pool = LendingPoolStorage::<T>::get(asset_pool.clone());
			ensure!(pool.is_active() == true, Error::<T>::LendingPoolAlreadyDeactivated);

			// ok now we can de-activate it
			LendingPoolStorage::<T>::mutate(asset_pool, |v| v.activated = false);
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

		/// Returns the the block's timestamp in seconds as u64
		fn now_in_seconds() -> u64 {
			core::time::Duration::from_millis(T::Time::now().saturated_into::<u64>())
				.as_secs()
				.saturated_into::<u64>()
		}

		/// Returns the amount of asset equivalent to the collateral
		/// checks if price of collateral asset available in terms of asset then
		/// return `price * collateral_balance `
		/// else
		/// check if price of asset available in terms of collateral asset then
		/// return `collateral_balance / price`
		/// else fallback to a common base asset
		/// get the prices of both assets in terms of asset 0 (USDT)  and
		/// return `collateral_asset_price * collateral_balance / asset_price`
		/// else
		/// return error `AssetPriceNotSet`
		fn get_equivalent_asset_amount(
			_who: &T::AccountId,
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
					AssetPrices::<T>::get((asset, 0)).ok_or(Error::<T>::AssetPriceNotSet)?;
				let collateral_price = AssetPrices::<T>::get((collateral_asset, 0))
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

		/// Returns the amount of collateral asset to be released on partila repayement
		/// Returns release_amount = pay / repayable_balance * collateral_balance
		fn get_release_collateral_amount(
			payment: AssetBalanceOf<T>,
			total_due: AssetBalanceOf<T>,
			collateral_balance: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let f_payment = FixedU128::from(payment.saturated_into::<u128>());
			let f_total_due = FixedU128::from(total_due.saturated_into::<u128>());
			let f_collateral_balance = FixedU128::from(collateral_balance.saturated_into::<u128>());

			let amount = f_payment
				.checked_div(&f_total_due)
				.ok_or(Error::<T>::OverflowError)?
				.checked_mul(&f_collateral_balance)
				.ok_or(Error::<T>::OverflowError)?
				.into_inner()
				.saturated_into();

			Ok(amount)
		}
	}
}
