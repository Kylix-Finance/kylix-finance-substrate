use crate::{tests::mock::*, AssetPool, Error, Event, LendingPool};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::FixedU128;

#[test]
fn deposit_tokens_into_pool() {
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
fn deposit_tokens_into_inactive_pool_fails() {
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
fn redeem_all_tokens_from_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![(DOT, ALICE, 1_000_000), (DOT, BOB, 1_000_000)])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 1000);
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 500));
			assert_ok!(TemplateModule::withdraw(RuntimeOrigin::signed(BOB), DOT, 500));

			System::assert_last_event(Event::DepositWithdrawn { who: BOB, balance: 500 }.into());
		});
}

#[test]
fn redeem_partial_tokens_from_pool() {
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

#[test]
fn borrow_maximum_allowed_tokens_from_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 1000);
			let bob_initial_ksm_balance = Fungibles::balance(KSM, &BOB);
			let bob_initial_dot_balance = Fungibles::balance(DOT, &BOB);

			let pallet_initial_dot_balance = get_pallet_balance(DOT);
			let pallet_initial_ksm_balance = get_pallet_balance(KSM);

			// Verify that the DOT pool exists
			let asset_pool = AssetPool::<Test>::from(DOT);
			assert!(TemplateModule::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			let price = FixedU128::from_rational(1, 1);
			let ksm_collateral_amount = 1_000;
			let dot_borrow_amount = 500;
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				KSM,
				price
			));

			// Should be unable to borrow more than 50% of the liquidity
			assert_noop!(
				TemplateModule::borrow(
					RuntimeOrigin::signed(BOB),
					DOT,                   // asset to borrow
					dot_borrow_amount + 1, // amount to borrow
					KSM,                   // collateral asset
					ksm_collateral_amount  // collateral amount
				),
				Error::<Test>::NotEnoughCollateral
			);

			// BOB borrows 500 DOT using 1000 KSM as collateral
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                   // asset to borrow
				dot_borrow_amount,     // amount to borrow
				KSM,                   // collateral asset
				ksm_collateral_amount  // collateral amount
			));

			// Check if the borrow event was emitted
			System::assert_last_event(
				Event::DepositBorrowed { who: BOB, balance: dot_borrow_amount }.into(),
			);

			// - Verify BOB's DOT balance increased by 500
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount
			);
			// - Verify BOB's KSM balance decreased by 1000
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance + dot_borrow_amount);

			// - Verify the lending pool's state changed correctly
			assert_eq!(get_pallet_balance(DOT), pallet_initial_dot_balance - dot_borrow_amount);
			assert_eq!(get_pallet_balance(KSM), pallet_initial_ksm_balance + ksm_collateral_amount);
		});
}

#[test]
fn borrow_partial_amount_of_tokens_from_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 2_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 1000);
			let bob_initial_ksm_balance = Fungibles::balance(KSM, &BOB);
			let bob_initial_dot_balance = Fungibles::balance(DOT, &BOB);

			let pallet_initial_dot_balance = get_pallet_balance(DOT);
			let pallet_initial_ksm_balance = get_pallet_balance(KSM);

			// Verify that the DOT pool exists
			let asset_pool = AssetPool::<Test>::from(DOT);
			assert!(TemplateModule::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			// Set price: 1 DOT = 0.1 KSM
			let price = FixedU128::from_rational(1, 10);
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				KSM,
				price
			));

			// First borrow: partial amount
			let ksm_collateral_amount_1 = 500;
			let dot_borrow_amount_1 = 250; // Assuming 50% collateral factor
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,
				dot_borrow_amount_1,
				KSM,
				ksm_collateral_amount_1
			));

			// Check if the first borrow event was emitted
			System::assert_last_event(
				Event::DepositBorrowed { who: BOB, balance: dot_borrow_amount_1 }.into(),
			);

			// Verify balances after first borrow
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount_1
			);
			assert_eq!(
				Fungibles::balance(DOT, &BOB),
				bob_initial_dot_balance + dot_borrow_amount_1
			);
			assert_eq!(get_pallet_balance(DOT), pallet_initial_dot_balance - dot_borrow_amount_1);
			assert_eq!(
				get_pallet_balance(KSM),
				pallet_initial_ksm_balance + ksm_collateral_amount_1
			);

			// Second borrow: another partial amount with additional collateral
			let ksm_collateral_amount_2 = 500;
			let dot_borrow_amount_2 = 250; // Assuming 50% collateral factor
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,
				dot_borrow_amount_2,
				KSM,
				ksm_collateral_amount_2
			));

			// Check if the second borrow event was emitted
			System::assert_last_event(
				Event::DepositBorrowed { who: BOB, balance: dot_borrow_amount_2 }.into(),
			);

			// Final balance checks
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount_1 - ksm_collateral_amount_2
			);
			assert_eq!(
				Fungibles::balance(DOT, &BOB),
				bob_initial_dot_balance + dot_borrow_amount_1 + dot_borrow_amount_2
			);
			assert_eq!(
				get_pallet_balance(DOT),
				pallet_initial_dot_balance - dot_borrow_amount_1 - dot_borrow_amount_2
			);
			assert_eq!(
				get_pallet_balance(KSM),
				pallet_initial_ksm_balance + ksm_collateral_amount_1 + ksm_collateral_amount_2
			);
		});
}

#[test]
fn repay_all_borrowed_tokens_to_pool() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, BOB, 1_000_000),
			(DOT, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 1000);

			// Record initial balances
			let bob_initial_ksm_balance = Fungibles::balance(KSM, &BOB);
			let bob_initial_dot_balance = Fungibles::balance(DOT, &BOB);
			let pallet_initial_dot_balance = get_pallet_balance(DOT);
			let pallet_initial_ksm_balance = get_pallet_balance(KSM);

			// Verify that the DOT pool exists
			let asset_pool = AssetPool::<Test>::from(DOT);
			assert!(TemplateModule::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			// Set price: 1 DOT = 1 KSM for simplicity
			let price = FixedU128::from_rational(1, 1);
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				KSM,
				price
			));

			// Borrow 500 DOT using 1000 KSM as collateral
			let ksm_collateral_amount = 1000;
			let dot_borrow_amount = 500;
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,
				dot_borrow_amount,
				KSM,
				ksm_collateral_amount
			));

			// Verify balances after borrowing
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount
			);
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance + dot_borrow_amount);

			// Repay all borrowed DOT
			assert_ok!(TemplateModule::repay(
				RuntimeOrigin::signed(BOB),
				DOT,
				dot_borrow_amount,
				KSM
			));

			// Check if the repay event was emitted
			System::assert_last_event(
				Event::DepositRepaid { who: BOB, balance: dot_borrow_amount }.into(),
			);

			// Verify final balances
			// BOB's KSM balance should be back to initial (collateral returned)
			assert_eq!(Fungibles::balance(KSM, &BOB), bob_initial_ksm_balance);
			// BOB's DOT balance should be back to initial
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance);
			// Pallet's DOT balance should be back to initial
			assert_eq!(get_pallet_balance(DOT), pallet_initial_dot_balance);
			// Pallet's KSM balance should be back to initial
			assert_eq!(get_pallet_balance(KSM), pallet_initial_ksm_balance);

			// Verify that BOB can't repay more (since the loan is fully repaid)
			assert_noop!(
				TemplateModule::repay(RuntimeOrigin::signed(BOB), DOT, 1, KSM),
				Error::<Test>::LoanDoesNotExists
			);
		});
}
