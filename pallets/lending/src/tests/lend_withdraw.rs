use crate::{tests::mock::*, AssetPool, Error, Event, LendingPoolStorage};
use frame_support::{assert_noop, assert_ok};

#[test]
fn test_supply_succeeds_for_activated_lending_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			let initial_dot_balance = Fungibles::balance(DOT, &ALICE);
			let pallet_initial_dot_balance = get_pallet_balance(DOT);
			let amount = 1_000;
			let supply_amount = 500;
			assert_ok!(TemplateModule::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				amount
			));
			assert_noop!(
				TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, supply_amount),
				Error::<Test>::LendingPoolNotActive
			);

			TemplateModule::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT).unwrap();
			System::assert_last_event(
				Event::LendingPoolActivated { who: ALICE, asset: DOT }.into(),
			);
			// Check the pool storage for activated bool
			let asset_pool = AssetPool::<Test>::from(DOT);
			let pool = LendingPoolStorage::<Test>::get(&asset_pool).unwrap();
			assert!(pool.activated);
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, supply_amount),);
			// Check supply events
			System::assert_last_event(
				Event::DepositSupplied { who: ALICE, asset: DOT, balance: supply_amount }.into(),
			);
			assert!(System::events().iter().any(|record| record.event ==
				Event::LPTokenMinted {
					who: ALICE,
					asset: LENDING_POOL_TOKEN,
					balance: amount
				}
				.into()));
			// Check final balances
			assert_eq!(
				Fungibles::balance(DOT, &ALICE),
				initial_dot_balance - amount - supply_amount
			);
			assert_eq!(
				get_pallet_balance(DOT),
				pallet_initial_dot_balance + amount + supply_amount
			);
			assert_eq!(Fungibles::balance(LENDING_POOL_TOKEN, &ALICE), amount + supply_amount);
			// Check pool storage for updated values
			let pool = LendingPoolStorage::<Test>::get(&asset_pool).unwrap();
			assert_eq!(pool.reserve_balance, amount + supply_amount);
		});
}

#[test]
fn test_supply_fails_with_zero_amount() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			let amount = 1_000;
			assert_ok!(TemplateModule::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				amount
			));
			TemplateModule::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT).unwrap();
			assert_noop!(
				TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, 0),
				Error::<Test>::InvalidLiquiditySupply
			);
		});
}

#[test]
fn test_supply_fails_with_insufficient_balance() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			let amount = 1_000;
			assert_ok!(TemplateModule::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				amount
			));
			TemplateModule::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT).unwrap();
			assert_noop!(
				TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, amount),
				Error::<Test>::NotEnoughLiquiditySupply
			);
		});
}

#[test]
fn test_supply_fails_for_nonexistent_lending_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			assert_noop!(
				TemplateModule::supply(RuntimeOrigin::signed(ALICE), DOT, 1_000),
				Error::<Test>::LendingPoolDoesNotExist
			);
		});
}

#[test]
fn test_supply_fails_for_inactive_lending_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000)])
		.build()
		.execute_with(|| {
			assert_ok!(TemplateModule::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
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
fn test_supply_succeeds_for_active_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 1000);
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 500));

			System::assert_last_event(
				Event::DepositSupplied { who: BOB, asset: DOT, balance: 500 }.into(),
			);
		});
}

#[test]
fn test_supply_fails_for_inactive_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			assert_ok!(TemplateModule::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN,
				DOT,
				1000
			));
			assert_noop!(
				TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 500),
				Error::<Test>::LendingPoolNotActive
			);
		});
}

#[test]
fn test_withdraw_all_tokens_succeeds() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 1000);
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 500));

			let initial_dot_balance = Fungibles::balance(DOT, &BOB);
			let pallet_initial_dot_balance = get_pallet_balance(DOT);
			let withdraw_amount = 500;
			assert_ok!(TemplateModule::withdraw(RuntimeOrigin::signed(BOB), DOT, withdraw_amount));

			System::assert_last_event(
				Event::DepositWithdrawn { who: BOB, balance: withdraw_amount }.into(),
			);

			// Check final balances
			assert_eq!(Fungibles::balance(DOT, &BOB), initial_dot_balance + withdraw_amount);
			assert_eq!(get_pallet_balance(DOT), pallet_initial_dot_balance - withdraw_amount);

			// Check that LP token was burned
			assert_eq!(Fungibles::balance(LENDING_POOL_TOKEN, &BOB), 0);

			// Check that reserve balance has been updated in storage
			let asset_pool = AssetPool::<Test>::from(DOT);
			let pool = LendingPoolStorage::<Test>::get(&asset_pool).unwrap();
			assert_eq!(pool.reserve_balance, 1_000);
		});
}

#[test]
fn test_withdraw_fails_with_insufficient_eligibility() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 1000);
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 500));
			assert_noop!(
				TemplateModule::withdraw(RuntimeOrigin::signed(BOB), DOT, 501),
				Error::<Test>::NotEnoughEligibleLiquidityToWithdraw
			);
		});
}

#[test]
fn test_withdraw_fails_with_insufficient_reserve() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 1000);
			assert_noop!(
				TemplateModule::withdraw(RuntimeOrigin::signed(BOB), DOT, 1500),
				Error::<Test>::NotEnoughLiquiditySupply
			);
		});
}

#[test]
fn test_withdraw_fails_with_zero_amount() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 1000);
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 500));
			assert_noop!(
				TemplateModule::withdraw(RuntimeOrigin::signed(BOB), DOT, 0),
				Error::<Test>::InvalidLiquidityWithdrawal
			);
		});
}

#[test]
fn test_withdraw_fails_for_nonexistent_lending_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			assert_noop!(
				TemplateModule::withdraw(RuntimeOrigin::signed(BOB), DOT, 500),
				Error::<Test>::LendingPoolDoesNotExist
			);
		});
}

#[test]
fn test_partial_withdraw_succeeds() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 1000);
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 500));
			assert_ok!(TemplateModule::withdraw(RuntimeOrigin::signed(BOB), DOT, 250));

			System::assert_last_event(Event::DepositWithdrawn { who: BOB, balance: 250 }.into());
		});
}
