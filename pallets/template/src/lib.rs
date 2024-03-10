#![cfg_attr(not(feature = "std"), no_std)]

///! # The Lending pallet
///!
///! ## Overview
///!
///! The Lending pallet is responsible for managing the lending pools and the assets.
///!
///! The lending pallet adopts a protocol similar to Compound V2 for its lending operations,
///! leveraging a pool-based approach to aggregate assets from all users.
///!  
///! Interest rates adjust dynamically in response to the supply and demand conditions.
///! Additionally, for every lending positions a new token is minted, thus enabling the transfer of
///! ownership.
///!
///! Implemented Extrinsics:
///!
///! 1. create_lending_pool
///! 2. activate_lending_pool
///! 3. supply
///! 4. withdraw
///! 5. borrow
///! 6. repay
///! 7. claim_rewards
///! 8. deactivate_lending_pool
///! 9. update_pool_rate_model
///! 10. update_pool_kink
///!
///! Use case

use frame_support::{
	pallet_prelude::*,
	traits::{fungible, fungibles},
	sp_runtime::{FixedU128, Permill}
};
pub use pallet::*;

/// Account Type Definition
pub type AccountOf<T> = <T as frame_system::Config>::AccountId;

/// Asset Id
pub type AssetIdOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<AccountOf<T>>>::AssetId;

/// Fungible Balance
pub type AssetBalanceOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<AccountOf<T>>>::Balance;

/// Native Balance
pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<AccountOf<T>>>::Balance;

//pub type BalanceOf<T> = <T as currency::Config>::Balance;

pub type Rate = FixedU128;
pub type Ratio = Permill;

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
	use frame_system::pallet_prelude::*;
	use frame_support::traits::tokens::Preservation;
	use frame_support::PalletId;
	use frame_support::sp_runtime::traits::{AccountIdConversion, Zero, One};
	use frame_support::traits::fungibles::{Inspect,Mutate,Create};
	use frame_support::{
		traits::{
			fungible::{self},
			fungibles::{self},
		}, DefaultNoBound
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The pallet's config trait.
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

		/// The origin which can add or remove LendingPools and update LendingPools (interest rate
		/// model, kink, activate, deactivate). TODO
		// type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	/// The AssetPool definition. Used as the Key in the lending pool storage
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialOrd, DefaultNoBound)]
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
	/// A struct to hold the LendingPool and all its properties, 
	/// used as Value in the lending pool storage
	/// 
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialOrd, DefaultNoBound)]
	#[scale_info(skip_type_params(T))]
	pub struct LendingPool<T: Config> {
		
		pub id: AssetIdOf<T>, // the lending pool id
		pub balance: AssetBalanceOf<T>, // the not-yet-borrowed balance of the lending pool
		pub borrowed_balance: AssetBalanceOf<T>,
		
		pub activated: bool,
		pub borrow_index: Rate,
		pub exchange_rate: Rate,
		pub borrow_rate: Rate,
		pub supply_rate: Rate,
		pub utilisation_ratio: Rate,

		pub liquidation_threshold: Permill,

		 /* minted token, kink */
	}
	impl<T: Config> LendingPool<T> {

		// let's create a default reserve lending pool 
		pub fn from(id: AssetIdOf<T>, balance: AssetBalanceOf<T>) -> Self {
			LendingPool { 
				id, 
				balance,
				borrowed_balance: AssetBalanceOf::<T>::zero(),
				activated: false,
				borrow_index : Rate::one(),
			 	exchange_rate: Rate::one(), 
				borrow_rate: Rate::zero(),
				supply_rate: Rate::zero(),
				utilisation_ratio: Rate::zero(),
				liquidation_threshold: Permill::one(), // should be 80%
			}
		}

		pub fn is_empty(&self) -> bool {
			self.balance.cmp(&BalanceOf::<T>::zero()).is_eq()
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
	///
	#[pallet::storage]
	#[pallet::getter(fn reserve_pools)]
	pub type LendingPoolStorage<T> =
		StorageMap<_, Blake2_128Concat, AssetPool<T>, LendingPool<T>, ValueQuery>;

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
		LendingPoolActivated { who: T::AccountId, asset : AssetIdOf<T> },
		LendingPoolDeactivated { who: T::AccountId, asset : AssetIdOf<T> },
		LendingPoolRateModelUpdated { who: T::AccountId, asset : AssetIdOf<T> },
		LendingPoolKinkUpdated { who: T::AccountId, asset : AssetIdOf<T> },
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
		/// The balance amount supplied is not valid
		InvalidLiquiditySupply,
		/// The user has not enough liquidity
		NotEnoughLiquiditySupply,
		/// Lending Pool is empty
		LendingPoolIsEmpty,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		
		/// The `create_lending_pool` function allows a user to Create a new Lending pool
		/// and then supply some liquidity. Given an asset and its amount, it creates a 
		/// new lending pool if it does not already exist and adds the provided liquidity
		/// to the existing pool. The user will receive LP tokens in return.
		///
		/// # Arguments
		///
		/// * `origin` - The origin caller of this function. This should be signed by the user
		///   that creates the lending pool and add some liquidity.
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
		/// * `LendingPoolAdded(asset_a)` if a new liquidity pool was created.
		/// * `DepositSupplied(asset_a, asset_b, amount_a, amount_b)` after the liquidity has been
		///   successfully added.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn create_lending_pool(origin: OriginFor<T>, id: AssetIdOf<T>, asset : AssetIdOf<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_create_lending_pool(&who, id, asset, balance)?;
			Self::deposit_event(Event::LendingPoolAdded { who : who.clone(), asset });
			Self::deposit_event(Event::DepositSupplied { who, asset, balance });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		pub fn activate_lending_pool(
			origin: OriginFor<T>,
			asset : AssetIdOf<T>
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_activate_lending_pool(asset)?;
			Self::deposit_event(Event::LendingPoolActivated { who, asset });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(Weight::default())]
		pub fn supply(origin: OriginFor<T>, asset : AssetIdOf<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_supply(&who, asset)?;
			Self::deposit_event(Event::DepositSupplied { who, asset, balance });
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(Weight::default())]
		pub fn withdraw(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::DepositWithdrawn { who, balance });
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(Weight::default())]
		pub fn borrow(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::DepositBorrowed { who, balance });
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(Weight::default())]
		pub fn repay(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
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

		#[pallet::call_index(7)]
		#[pallet::weight(Weight::default())]
		pub fn deactivate_lending_pool(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::RewardsClaimed { who, balance });
			Ok(())
		}


		#[pallet::call_index(8)]
		#[pallet::weight(Weight::default())]
		pub fn update_pool_rate_model(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::RewardsClaimed { who, balance });
			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(Weight::default())]
		pub fn update_pool_kink(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::RewardsClaimed { who, balance });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {

		// This method creates a NEW lending pool and mints LP tokens back to the user. 
		// At this very moment, the user is the first liquidity provider, therefore the number of LP tokens
		// minted is equal to assets deposited. 
		//
		pub fn do_create_lending_pool(
			who: &T::AccountId,
			id: AssetIdOf<T>,
			asset:  AssetIdOf<T>,
			balance: BalanceOf<T>
		) -> DispatchResult {
			
			// First, let's check the balance amount is valid
			ensure!(
				balance > BalanceOf::<T>::zero(),
				Error::<T>::InvalidLiquiditySupply
			);

			// Second, let's check the if user has enough liquidity
			let user_balance = T::Fungibles::balance(asset.clone(), who);
			ensure!(
				user_balance >= balance,
				Error::<T>::NotEnoughLiquiditySupply
			);
		
			// Now let's check if the pool is already existing, before creating a new one.
			let asset_pool = AssetPool::<T>::from(asset); 
			ensure!(
				!LendingPoolStorage::<T>::contains_key(&asset_pool), 
				Error::<T>::LendingPoolAlreadyExists
			);

			// Now we can safely create and store our lending pool with initial balance
			let asset_pool = AssetPool::from(asset);
			let lending_pool = LendingPool::<T>::from(asset, balance);
			LendingPoolStorage::<T>::insert(asset_pool, lending_pool);
		
			// let's transfers the tokens (asset) from the users account into pallet account 
			T::Fungibles::transfer(asset.clone(), who, &Self::account_id(), balance, Preservation::Expendable)?;
		
			// checks if the liquidity token already exists and if not create it
			if !T::Fungibles::asset_exists(id.clone()) {
				T::Fungibles::create(id.clone(), Self::account_id(), true, One::one())?;
			}
	
			// mints the lp tokens into the users account
			T::Fungibles::mint_into(id, &who, balance)?;
			
			Ok(())
		}

		pub fn do_activate_lending_pool(
			asset: AssetIdOf<T>
		) -> DispatchResult {
			
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
			LendingPoolStorage::<T>::mutate(asset_pool, |v| {
				v.activated = true
			});
			Ok(())
		}

		pub fn do_supply(who: &T::AccountId,
			asset:  AssetIdOf<T>
		) -> DispatchResult {
		
			Ok(())
		}

		/// This method returns the palled account id
		///
		/// This actually does computation. If you need to keep using it, 
		/// then make sure to cache the value and only call this once.
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}
}
