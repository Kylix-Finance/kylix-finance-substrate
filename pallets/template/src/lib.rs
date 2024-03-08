//! # Kylix = Lending pallet
//!
//! ## Overview
//!
//! The Lending pallet is responsible for managing the lending pools and the assets.
//!
//! The lending pallet adopts a protocol similar to Compound V2 for its lending operations, 
//! leveraging a pool-based approach to aggregate assets from all users.
//!  
//! Interest rates adjust dynamically in response to the supply and demand conditions. 
//! Additionally, for every lending positions a new token is minted, thus enabling the transfer of ownership.
//! 
//! Defined Extrinsics:
//! 
//! 1.  supply
//! 2.  withdraw
//! 3.  borrow
//! 4.  repay
//! 5.  claim_rewards
//! 6.  add_lending_pool
//! 7.  remove_lending_pool
//! 8.  activate_lending_pool
//! 9.  deactivate_lending_pool
//! 10. update_pool_rate_model
//! 11. update_pool_kink
//!
//! Use case

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;
use frame_support::traits::fungibles;

pub type AssetIdOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
	<T as frame_system::Config>::AccountId,
>>::AssetId;

pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<<T as frame_system::Config>::AccountId, >>::Balance; // Native Balance

pub type AssetBalanceOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
	<T as frame_system::Config>::AccountId,
>>::Balance; 

//pub type BalanceOf<T> = <T as currency::Config>::Balance;

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
	//use frame_benchmarking::v2::assert_type_eq_all;
	use super::*;
	use frame_support::traits::fungibles;
	use frame_support::pallet_prelude::*;
	use frame_system::{pallet_prelude::*, AccountInfo};
	use frame_support::{
		traits::{
			fungible::{self},
		
		}, DefaultNoBound, PalletId
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

		/// The origin which can add or remove LendingPools and update LendingPools (interest rate model, kink, activate, deactivate).
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;		
	}

	/// The AssetPool definition. Used as the Key in the lending pool storage
	pub struct AssetPool<T: Config> {
		asset: AssetIdOf<T>,
	}

	// Definition of the Lending Pool Reserve Entity
	// A handy Struct to hold the LendingPool and all its properties.
	// Used as Value in the lending pool storage
	pub struct LendingPool<T: Config> {
		pub id: AssetIdOf<T>,                    // the lending pool id
		pub balance_free: AssetBalanceOf<T>,	 // the not-yet-borrowed balance of the lending pool
		// minted tokens
		// rate model
		// kink
		//pub balance_locked: AssetBalanceOf<T>,
	}
	impl<T: Config> LendingPool<T> {
		pub fn from(id: AssetIdOf<T>, balance_free: AssetBalanceOf<T>) -> Self {
			LendingPool { id, balance_free }
		}
	}

	// PolyLend runtime storage items
	//
	// Lending pools defined for the assets
	//
	// StorageMap AssetPool { AssetId } => LendingPool { PoolId, Balance }
	#[pallet::storage]
	#[pallet::getter(fn reserve_pools)]
	pub type ReservePools<T> = StorageMap<_, Blake2_128Concat, AssetPool<T>, LendingPool<T>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {

		DepositSupplied { balance: u32, who: T::AccountId },
		DepositWithdrawn { balance: u32, who: T::AccountId },
		DepositBorrowed { balance: u32, who: T::AccountId },
		DepositRepaid { balance: u32, who: T::AccountId },
		RewardsClaimed { balance: u32, who: T::AccountId },
		LendingPoolAdded { who: T::AccountId },
		LendingPoolRemoved { who: T::AccountId },
		LendingPoolActivated { who: T::AccountId },
		LendingPoolDeactivated { who: T::AccountId },
		LendingPoolRateModelUpdated { who: T::AccountId },
		LendingPoolKinkUpdated { who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Lending Pool does not exist
		LendingPoolDoesNotExist,
		/// Lending Pool already activated
		LendingPoolAlreadyActivated,
		/// Lending Pool already deactivated
		LendingPoolAlreadyDeactivated,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Create a new Lending pool and then supply some liquidity
		///
		/// - `origin`: signed origin call
		/// - `liquidity_pool_id`: liquidity pool id.
		/// - `asset_id_a`: token asset id A.
		/// - `asset_id_b`: token asset id B.
		/// - `amount_a`: Balance amount of asset A.
		/// - `amount_b`: Balance amount of asset B.
		/// 
		/// It generates 2 events
		/// - `Event::LiquidityPoolCreated`
		/// - `Event::AddedLiquidity`
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn do_add_lending_pool(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::create_lending_pool(balance);
			Self::deposit_event(Event::LendingPoolAdded { who });
			Self::deposit_event(Event::DepositSupplied { balance, who });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_activate_lending_pool(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::LendingPoolActivated { who });
			
			Ok(())
		}
	
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_supply(origin: OriginFor<T>, balance: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::DepositSupplied { balance, who });
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_withdraw(origin: OriginFor<T>, something: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::SomethingStored { something, who });
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_borrow(origin: OriginFor<T>, something: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::SomethingStored { something, who });
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_repay(origin: OriginFor<T>, something: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::SomethingStored { something, who });
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_claim_rewards(origin: OriginFor<T>, something: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::deposit_event(Event::SomethingStored { something, who });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// The account ID of the Lending pot.
		///
		/// This actually does computation. If you need to keep using it, then make sure you cache the
		/// value and only call this once.
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}
}
