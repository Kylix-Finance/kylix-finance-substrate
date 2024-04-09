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
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{FixedU128, Permill, SaturatedConversion},
	traits::{fungible, fungibles},
};
pub use pallet::*;

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
	use frame_support::{
		sp_runtime,
		sp_runtime::traits::{
			AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero,
		},
		traits::{
			fungible::{self},
			fungibles::{
				Create, Inspect, Mutate, {self},
			},
			tokens::Preservation,
		},
		DefaultNoBound, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::FixedPointNumber;

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
	}
	impl<T: Config> LendingPool<T> {
		// let's create a default reserve lending pool
		pub fn from(
			id: LendingPoolId,
			lend_token_id: AssetIdOf<T>,
			balance: AssetBalanceOf<T>,
		) -> Self {
			LendingPool {
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
			}
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

	impl<T: Config> UnderlyingAsset<T> {
		pub fn supply_index(&self) -> Result<Rate, Error<T>> {
			// TODO: implement
			Ok(Rate::one())
		}
		/// Calculates scaled deposit as
		/// scaled_deposit = deposit / supply_index
		pub fn scaled_deposit(
			&self,
			deposit: AssetBalanceOf<T>,
		) -> Result<AssetBalanceOf<T>, Error<T>> {
			let scaled_deposit = FixedU128::from_inner(deposit.saturated_into())
				.checked_div(&self.supply_index()?)
				.ok_or(Error::<T>::OverflowError)?
				.into_inner()
				.saturated_into();
			Ok(scaled_deposit)
		}
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
		/// * `LendingPoolAdded(asset_a)` if a new lending pool was created.
		/// * `DepositSupplied(asset_a, asset_b, amount_a, amount_b)` after the liquidity has been
		///   successfully added.
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
		/// * `LendingPoolActivated(asset_a)` if the lending pool was activated.
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
			balance: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_borrow(&who, asset, balance)?;
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
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_repay(&who, asset, balance)?;
			Self::deposit_event(Event::DepositRepaid { who, balance });
			Ok(())
		}

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
		/// * `LendingPoolDeactivated(asset_a)` if the lending pool was deactivated.
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

		#[pallet::call_index(8)]
		#[pallet::weight(Weight::default())]
		pub fn update_pool_rate_model(origin: OriginFor<T>, asset: AssetIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::LendingPoolRateModelUpdated { who, asset });
			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(Weight::default())]
		pub fn update_pool_kink(origin: OriginFor<T>, asset: AssetIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::LendingPoolKinkUpdated { who, asset });
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
			let lending_pool = LendingPool::<T>::from(id, asset, balance);

			LendingPoolStorage::<T>::insert(asset_pool, &lending_pool);

			// ...and save the information related to the underlying asset.
			//
			// TODO: It is not convenient to instantiate a Default here that anyway needs to be
			// updated at a later point. We should instead have a way to define the properties of
			// the underlying asset from outside.
			let underlying_asset = UnderlyingAsset::<T> {
				underlying_asset_id: asset,
				last_accrued_interest: 0,
				total_borrowed: BalanceOf::<T>::zero(),
				total_supply: BalanceOf::<T>::zero(),
				borrow_index: Rate::one(),
				exchange_rate: Rate::one(),
				borrow_rate: Rate::one(),
				supply_rate: Rate::one(),
				utilization_rate: Rate::one(),
				reward_supply_speed: BalanceOf::<T>::zero(),
				reward_borrow_speed: BalanceOf::<T>::zero(),
				reward_accrued: BalanceOf::<T>::zero(),
			};

			// Let's calculate the amount of LP tokens to mint
			// pro quota based on the total supply following the formula:
			//
			// minted_tokens = deposit * total_issuance / total_liquidity

			let total_issuance = T::Fungibles::total_issuance(asset.clone());
			let minted_tokens = total_issuance
				.checked_mul(&balance)
				.ok_or(Error::<T>::OverflowError)?
				.checked_div(&lending_pool.reserve_balance)
				.ok_or(Error::<T>::OverflowError)?;

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

			let scaled_minted_tokens = underlying_asset.scaled_deposit(minted_tokens)?;
			// mints the lp tokens into the users account
			T::Fungibles::mint_into(id, &who, scaled_minted_tokens)?;
			// Create suppliers supply_index
			let supply_index = SupplyIndex::from(
				underlying_asset.supply_index()?,
				underlying_asset.last_accrued_interest,
			);
			SupplyIndexStorage::<T>::insert((who, asset), supply_index);

			UnderlyingAssetStorage::<T>::insert(lending_pool.lend_token_id, underlying_asset);

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

			// Ok, now let's calculate the amount of LP tokens to mint
			// pro quota based on the total supply and the total liquidity in the pool
			// following the formula:
			//
			// minted_tokens = deposit * total_issuance / total_liquidity

			let total_issuance = T::Fungibles::total_issuance(asset.clone());
			let minted_tokens = total_issuance
				.checked_mul(&balance)
				.ok_or(Error::<T>::OverflowError)?
				.checked_div(&pool.reserve_balance)
				.ok_or(Error::<T>::OverflowError)?;

			// let's transfers the tokens (asset) from the users account into pallet account
			T::Fungibles::transfer(
				asset.clone(),
				who,
				&Self::account_id(),
				balance,
				Preservation::Expendable,
			)?;

			// mints the LP tokens into the users account
			T::Fungibles::mint_into(pool.id, &who, minted_tokens)?;

			pool.reserve_balance =
				pool.reserve_balance.checked_add(&balance).ok_or(Error::<T>::OverflowError)?;

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

			// let's check if the user is actually elegible to withdraw!
			let lp_tokens = T::Fungibles::balance(pool.id.clone(), &who);
			ensure!(lp_tokens >= balance, Error::<T>::NotEnoughElegibleLiquidityToWithdraw);

			// Get the current reserves
			/*let reserve_user = T::Fungibles::balance(asset.clone(), &Self::account_id());

			// Calculate the
			let total_issuance = T::Fungibles::total_issuance(pool.id.clone());

			// Adjusted liquidity calculation
			let interest = pool.interest_rate_model()
				.mul_ceil(pool.borrowed_balance);
			*/
			//	let l_adj = self.total_deposits
			//		.checked_add(&interest)?;

			//	let share = Perbill::from_rational(amount_lp_tokens_burn,
			// self.total_supply_lp_tokens); 	let tokens_to_return = share.mul_ceil(l_adj);

			pool.reserve_balance =
				pool.reserve_balance.checked_sub(&balance).ok_or(Error::<T>::OverflowError)?;

			// let's update the balances of the pool now
			LendingPoolStorage::<T>::set(&asset_pool, pool);

			Ok(())
		}

		///
		fn do_borrow(
			_who: &T::AccountId,
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

			// let's check if the pool is active
			let pool = LendingPoolStorage::<T>::get(asset_pool.clone());
			ensure!(pool.is_active() == true, Error::<T>::LendingPoolNotActive);

			// let's check the if the pool has enough liquidity
			ensure!(pool.reserve_balance >= balance, Error::<T>::NotEnoughLiquiditySupply);

			Ok(())
		}

		fn do_repay(
			_who: &T::AccountId,
			asset: AssetIdOf<T>,
			balance: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(balance > BalanceOf::<T>::zero(), Error::<T>::InvalidLiquidityWithdrawal);

			// let's check if our pool does exist
			let asset_pool = AssetPool::<T>::from(asset);
			ensure!(
				LendingPoolStorage::<T>::contains_key(&asset_pool),
				Error::<T>::LendingPoolDoesNotExist
			);

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

		// create and mint into an account
		pub fn create_and_mint(
			asset_id: AssetIdOf<T>,
			who: T::AccountId,
			asset_balance: AssetBalanceOf<T>,
		) -> DispatchResult {
			T::Fungibles::create(asset_id.clone(), Self::account_id(), true, One::one())?;
			T::Fungibles::mint_into(asset_id.clone(), &who, asset_balance)?;
			Ok(())
		}
	}
}
