use crate::{mock::*, Error, LendingPool, LendingPoolId};

use frame_support::{assert_noop, assert_ok};
//use frame_system::Origin;

use sp_runtime::{
	FixedU128,
	//traits::BadOrigin,
	Permill,
};

pub type Rate = FixedU128;
//pub type Ratio = Permill;

type Token = u32;
type SignedOrigin = u64;
type BalanceAmount = u128;

const ALICE: SignedOrigin = 1u64;
const BOB: SignedOrigin = 2u64;

const DOT: Token = 1u32;

const LENDING_POOL_ID: LendingPoolId = 0;

// Test helper for creating an account and minting a specific token
//scaled balance use scaled_deposit fn
fn setup_test_account(token: Token, address: u64, amount: BalanceAmount) {
	let _ = TemplateModule::update_and_mint(&ALICE, token, LENDING_POOL_ID, balance(token, ALICE), sp_runtime::FixedU128::from(1));
	let res = balance(token, address);
	assert_eq!(res, amount);
}

// Test helper for fetching am account Balance amount
fn balance(token: Token, address: u64) -> BalanceAmount {
	pallet_assets::Pallet::<Test>::balance(token, address)
}

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
	});
}

#[test]
fn test_the_default_utilisation_rate() {
	let pool: LendingPool<Test> = LendingPool::from(0, DOT, 10000).expect("failed");

	assert_eq!(pool.reserve_balance, 10000);
	assert_eq!(pool.borrowed_balance, 0);
	assert_eq!(pool.is_active(), false);
	assert_eq!(pool.is_empty(), false);

	let ut = pool.utilisation_ratio().unwrap();
	assert_eq!(ut, Permill::zero());

	let br = pool.borrow_interest_rate().unwrap();
	assert_eq!(br, Rate::from_float(0.0));
}

#[test]
fn test_utilisation_rate_with_some_supply_and_borrowing() {
	let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 5000).expect("failed");
	pool.borrowed_balance = 5000;

	println!("Test Pool1: {:#?}", pool);

	let ut = pool.utilisation_ratio().unwrap();
	assert_eq!(ut, Permill::from_percent(50)); // 5000/10000 = 50%

	let br = pool.borrow_interest_rate().unwrap();
	assert_eq!(br, Rate::from_float(0.045)); // 4.5%

	// 4.5% borrow interest rate for 50% utilisation rate.
	// it can be aslso verified visually from https://www.desmos.com/calculator/fnj0ctpqn9
}

#[test]
fn test_utilisation_rate_with_some_supply_and_borrowing2() {
	let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 1000).expect("failed");
	pool.borrowed_balance = 9000;

	let ut = pool.utilisation_ratio().unwrap();
	assert_eq!(ut, Permill::from_percent(90));
}

#[test]
fn test_supply_rate() {
	let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 5000).expect("failed");
	pool.borrowed_balance = 5000;

	println!("Test Reserve Factor: {:#?}", pool.reserve_factor);
	let reserved = Permill::from_percent(100) - (pool.reserve_factor);

	println!("Test Reserved: {:#?}", reserved);

	let ut = pool.supply_interest_rate().unwrap();
	assert_eq!(ut, Rate::from_float(0.02025)); // 20.25%
}

#[test]
fn test_supply_rate2() {
	let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 1000).expect("failed");
	pool.borrowed_balance = 9000;

	let ut = pool.supply_interest_rate().unwrap();
	assert_eq!(ut, Rate::from_float(0.018225)); // 18.225%
}

#[test]
fn try_to_supply_no_lending_pool() {
	new_test_ext().execute_with(|| {
		setup_test_account(DOT, ALICE, 1_000_000);

		// Supply
		assert_noop!(
			TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, 1_000),
			Error::<Test>::LendingPoolDoesNotExist
		);
	});
}

#[test]
fn try_to_supply_no_liquidity() {
	new_test_ext().execute_with(|| {
		// Supply
		assert_noop!(
			TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 1_000),
			Error::<Test>::NotEnoughLiquiditySupply
		);
	});
}

#[test]
fn try_to_create_lending_pool() {
	new_test_ext().execute_with(|| {
		setup_test_account(DOT, ALICE, 1_000_000);

		assert_ok!(TemplateModule::create_lending_pool(
			RuntimeOrigin::signed(ALICE),
			LENDING_POOL_ID,
			DOT,
			1_000
		));
	});
}

#[test]
fn try_to_create_lending_pool_and_supply_not_yet_active() {
	new_test_ext().execute_with(|| {
		setup_test_account(DOT, ALICE, 1_000_000);

		assert_ok!(TemplateModule::create_lending_pool(
			RuntimeOrigin::signed(ALICE),
			LENDING_POOL_ID,
			DOT,
			1_000
		));
		assert_noop!(
			TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, 1_000),
			Error::<Test>::LendingPoolNotActive
		);
	});
}

#[test]
fn try_to_create_lending_pool_and_supply_active() {
	new_test_ext().execute_with(|| {
		setup_test_account(DOT, ALICE, 1_000_000);

		assert_ok!(TemplateModule::create_lending_pool(
			RuntimeOrigin::signed(ALICE),
			LENDING_POOL_ID,
			DOT,
			1_000
		));
		//TemplateModule::activate_lending_pool(RuntimeOrigin::signed(ALICE),
		// LENDING_POOL_ID).unwrap();
		assert_noop!(
			TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, 1_000),
			Error::<Test>::LendingPoolNotActive
		);
	});
}
