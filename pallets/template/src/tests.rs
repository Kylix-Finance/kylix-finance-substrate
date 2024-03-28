
use crate::{mock::*, Error, Event, LendingPool};
use frame_support::{assert_noop, assert_ok};
use frame_system::Origin;
use sp_runtime::traits::BadOrigin;

type SignedOrigin = u64;

const ALICE: SignedOrigin = 1u64;
const BOB: SignedOrigin = 2u64;

const DOT: u32 = 1u32;

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

	let pool : LendingPool<Test> = LendingPool::from(DOT, 10000);
	
	let ut = pool.utilisation_ratio();
	assert_eq!(ut.unwrap().deconstruct(), 0);

	let br = pool.borrow_interest_rate();
	assert_eq!(br.unwrap().deconstruct(), 0);
}

#[test]
fn test_utilisation_rate_with_some_supply_and_borrowing() {

	let pool : LendingPool<Test> = LendingPool::from(DOT, 10000);
	
	let ut = pool.utilisation_ratio();
	assert_eq!(ut.unwrap().deconstruct(), 0);

	let br = pool.borrow_interest_rate();
	assert_eq!(br.unwrap().deconstruct(), 0);
	
}

#[test]
fn try_to_supply() {
	new_test_ext().execute_with(|| {

		// Supply
	//assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, 100));
		
		// Supply
	//	assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, 100));
		
		// Supply
	//	assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 100));
	
	});
}
