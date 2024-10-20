use crate::{tests::mock::*, AssetPool, Error, Event, LendingPool};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::{assert_eq_error_rate, Permill};

const NEW_ASSET: AssetId = 8888u32;

#[test]
fn test_create_lending_pool_succeeds_for_new_asset() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(NEW_ASSET, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			let amount = 1_000;
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				NEW_ASSET,
				amount
			));

			let asset_pool = AssetPool::<Test>::from(NEW_ASSET);
			assert!(Lending::reserve_pools(asset_pool).is_some());

			let events = System::events();
			assert!(events.iter().any(|record| record.event
				== Event::LendingPoolAdded { who: ALICE, asset: NEW_ASSET }.into()));
			assert!(events.iter().any(|record| record.event
				== Event::LiquiditySupplied { who: ALICE, asset: NEW_ASSET, balance: amount }
					.into()));
			assert!(events.iter().any(|record| record.event
				== Event::LPTokenMinted {
					who: ALICE,
					asset: LENDING_POOL_TOKEN,
					balance: amount
				}
				.into()));
		});
}

#[test]
fn test_create_lending_pool_fails_for_existing_asset() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			// First creation should succeed
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				1_000
			));

			// Second creation should fail
			assert_noop!(
				Lending::create_lending_pool(
					RuntimeOrigin::signed(ALICE),
					LENDING_POOL_TOKEN + 1,
					DOT,
					1_000
				),
				Error::<Test>::LendingPoolAlreadyExists
			);
		});
}

#[test]
fn test_create_lending_pool_fails_with_zero_amount() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			// Pool creation should fail
			assert_noop!(
				Lending::create_lending_pool(
					RuntimeOrigin::signed(ALICE),
					LENDING_POOL_TOKEN,
					DOT,
					0
				),
				Error::<Test>::InvalidLiquiditySupply
			);
		});
}

#[test]
fn test_create_lending_pool_fails_with_insufficient_balance() {
	ExtBuilder::default().with_endowed_balances(vec![]).build().execute_with(|| {
		// Pool creation should fail
		assert_noop!(
			Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				1_000
			),
			Error::<Test>::NotEnoughLiquiditySupply
		);
	});
}

#[test]
fn test_create_lending_pool_fails_with_existing_id() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (KSM, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			// First creation should succeed
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				1_000
			));

			// Creation with the same LENDING_POOL_TOKEN should fail
			assert_noop!(
				Lending::create_lending_pool(
					RuntimeOrigin::signed(ALICE),
					LENDING_POOL_TOKEN,
					KSM,
					1_000
				),
				Error::<Test>::IdAlreadyExists
			);
		});
}

#[test]
fn test_default_utilisation_rate() {
	ExtBuilder::default().build().execute_with(|| {
		let pool: LendingPool<Test> = LendingPool::from(0, DOT, 10000).expect("failed");

		assert_eq!(pool.reserve_balance, 10000);
		assert_eq!(pool.borrowed_balance, 0);
		assert_eq!(pool.is_active(), false);
		assert_eq!(pool.is_empty(), false);

		let ut = pool.utilisation_ratio().unwrap();
		assert_eq!(ut, Permill::zero());

		let br = pool.borrow_interest_rate().unwrap();
		assert_eq!(br, Rate::from_float(0.0));
	});
}

#[test]
fn test_utilisation_rate_with_partial_borrowing() {
	ExtBuilder::default().build().execute_with(|| {
		let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 5000).expect("failed");
		pool.borrowed_balance = 5000;

		let ut = pool.utilisation_ratio().unwrap();
		assert_eq!(ut, Permill::from_percent(50)); // 5000/10000 = 50%

		let br = pool.borrow_interest_rate().unwrap();
		let error_margin: Rate = Rate::from_float(0.001);
		assert_eq_error_rate!(br, Rate::from_float(0.03), error_margin);
	});
}

#[test]
fn test_utilisation_rate_with_high_borrowing() {
	ExtBuilder::default().build().execute_with(|| {
		let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 1000).expect("failed");
		pool.borrowed_balance = 9000;

		let ut = pool.utilisation_ratio().unwrap();
		assert_eq!(ut, Permill::from_percent(90));
	});
}

#[test]
fn test_supply_interest_rate_with_partial_borrowing() {
	ExtBuilder::default().build().execute_with(|| {
		let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 5000).expect("failed");
		let error_margin: Rate = Rate::from_float(0.001);
		pool.borrowed_balance = 5000;

		println!("Test Reserve Factor: {:#?}", pool.reserve_factor);
		let reserved = Permill::from_percent(100) - (pool.reserve_factor);

		println!("Test Reserved: {:#?}", reserved);

		let ut = pool.supply_interest_rate().unwrap();
		assert_eq_error_rate!(ut, Rate::from_float(0.013), error_margin);
	});
}

#[test]
fn test_supply_interest_rate_with_high_borrowing() {
	ExtBuilder::default().build().execute_with(|| {
		let mut pool: LendingPool<Test> = LendingPool::from(0, DOT, 1000).expect("failed");
		let error_margin: Rate = Rate::from_float(0.001);
		pool.borrowed_balance = 9000;

		let ut = pool.supply_interest_rate().unwrap();
		assert_eq_error_rate!(ut, Rate::from_float(0.024), error_margin);
	});
}

#[test]
fn test_activate_fails_for_non_existent_lending_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			// Supply
			assert_noop!(
				Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT),
				Error::<Test>::LendingPoolDoesNotExist
			);
		});
}

#[test]
fn test_activate_fails_for_already_activated_lending_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				1_000
			));

			Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT).unwrap();
			assert_noop!(
				Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT),
				Error::<Test>::LendingPoolAlreadyActivated
			);
		});
}
