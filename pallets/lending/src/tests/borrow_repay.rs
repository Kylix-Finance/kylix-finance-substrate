use crate::{tests::mock::*, AssetPool, Borrows, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::FixedU128;

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
			assert!(Lending::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			let price = FixedU128::from_rational(1, 1);
			let dot_borrow_amount = 500;
			assert_ok!(Lending::set_asset_price(RuntimeOrigin::signed(ALICE), DOT, KSM, price));

			let ksm_collateral_amount =
				Lending::estimate_collateral_amount(DOT, dot_borrow_amount, KSM).unwrap();

			// BOB borrows 500 DOT using 1000 KSM as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,               // asset to borrow
				dot_borrow_amount, // amount to borrow
				KSM
			));

			// Check if the borrow event was emitted
			System::assert_last_event(
				Event::Borrowed {
					who: BOB,
					borrowed_asset_id: DOT,
					borrowed_balance: dot_borrow_amount,
					collateral_asset_id: KSM,
					collateral_balance: ksm_collateral_amount,
				}
				.into(),
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
			assert!(Lending::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			// Set price: 1 DOT = 0.1 KSM
			let price = FixedU128::from_rational(1, 10);
			assert_ok!(Lending::set_asset_price(RuntimeOrigin::signed(ALICE), DOT, KSM, price));

			// First borrow: partial amount
			let dot_borrow_amount_1 = 250; // Assuming 50% collateral factor
			let ksm_collateral_amount_1 =
				Lending::estimate_collateral_amount(DOT, dot_borrow_amount_1, KSM).unwrap();
			assert_ok!(Lending::borrow(RuntimeOrigin::signed(BOB), DOT, dot_borrow_amount_1, KSM));

			// Check if the first borrow event was emitted
			System::assert_last_event(
				Event::Borrowed {
					who: BOB,
					borrowed_asset_id: DOT,
					borrowed_balance: dot_borrow_amount_1,
					collateral_asset_id: KSM,
					collateral_balance: ksm_collateral_amount_1,
				}
				.into(),
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
			let dot_borrow_amount_2 = 250; // Assuming 50% collateral factor
			let ksm_collateral_amount_2 =
				Lending::estimate_collateral_amount(DOT, dot_borrow_amount_2, KSM).unwrap();
			assert_ok!(Lending::borrow(RuntimeOrigin::signed(BOB), DOT, dot_borrow_amount_2, KSM));

			// Check if the second borrow event was emitted
			System::assert_last_event(
				Event::Borrowed {
					who: BOB,
					borrowed_asset_id: DOT,
					borrowed_balance: dot_borrow_amount_2,
					collateral_asset_id: KSM,
					collateral_balance: ksm_collateral_amount_2,
				}
				.into(),
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
			assert!(Lending::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			// Set price: 1 DOT = 1 KSM for simplicity
			let price = FixedU128::from_rational(1, 1);
			assert_ok!(Lending::set_asset_price(RuntimeOrigin::signed(ALICE), DOT, KSM, price));

			// Borrow 500 DOT using 1000 KSM as collateral
			let ksm_collateral_amount = 1000;
			let dot_borrow_amount = 500;
			assert_ok!(Lending::borrow(RuntimeOrigin::signed(BOB), DOT, dot_borrow_amount, KSM));

			// Verify balances after borrowing
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount
			);
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance + dot_borrow_amount);

			// Repay all borrowed DOT
			assert_ok!(Lending::repay(RuntimeOrigin::signed(BOB), DOT, dot_borrow_amount, KSM));

			// Check if the repay event was emitted
			System::assert_last_event(
				Event::Repaid {
					who: BOB,
					repaid_asset_id: DOT,
					repaid_balance: dot_borrow_amount,
					collateral_asset_id: KSM,
					collateral_balance: ksm_collateral_amount,
				}
				.into(),
			);

			// Verify final balances
			// // BOB's KSM balance should be back to initial (collateral returned)
			// assert_eq!(Fungibles::balance(KSM, &BOB), bob_initial_ksm_balance);
			// // BOB's DOT balance should be back to initial
			// assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance);
			// Pallet's DOT balance should be back to initial
			assert_eq!(get_pallet_balance(DOT), pallet_initial_dot_balance);
			// Pallet's KSM balance should be back to initial
			assert_eq!(get_pallet_balance(KSM), pallet_initial_ksm_balance);
		});
}

#[test]
fn partial_repay_of_borrowed_tokens() {
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
			assert!(Lending::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			// Set price: 1 DOT = 1 KSM for simplicity
			let price = FixedU128::from_rational(1, 1);
			assert_ok!(Lending::set_asset_price(RuntimeOrigin::signed(ALICE), DOT, KSM, price));

			// Borrow 500 DOT using 1000 KSM as collateral
			let ksm_collateral_amount = 1000;
			let dot_borrow_amount = 500;
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,               // asset to borrow
				dot_borrow_amount, // amount to borrow
				KSM
			));

			// Verify balances after borrowing
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount
			);
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance + dot_borrow_amount);

			// Partial repayment: BOB repays 200 DOT
			let repayment_amount = 200;
			assert_ok!(Lending::repay(RuntimeOrigin::signed(BOB), DOT, repayment_amount, KSM));

			// Check if the repay event was emitted
			System::assert_last_event(
				Event::Repaid {
					who: BOB,
					repaid_asset_id: DOT,
					repaid_balance: repayment_amount,
					collateral_asset_id: KSM,
					collateral_balance: 400,
				}
				.into(),
			);

			// Verify BOB's DOT balance decreased by 200
			assert_eq!(
				Fungibles::balance(DOT, &BOB),
				bob_initial_dot_balance + dot_borrow_amount - repayment_amount
			);

			// Calculate expected collateral release
			// Released collateral = repayment_amount / total_borrowed * collateral_balance
			let expected_collateral_release = (repayment_amount as u128 *
				ksm_collateral_amount as u128) /
				dot_borrow_amount as u128;
			let expected_collateral_release = expected_collateral_release;

			// Verify BOB's KSM balance increased by released collateral
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount + expected_collateral_release
			);

			// Verify remaining loan in Borrows storage
			let remaining_loan = Borrows::<Test>::get((BOB, DOT, KSM)).expect("Loan should exist");

			// Expected remaining borrowed balance
			let expected_remaining_borrowed = dot_borrow_amount - repayment_amount;

			// Since borrowed_balance is stored as scaled value, and borrow_index is 1 (no
			// interest), the borrowed_balance should match expected_remaining_borrowed
			assert_eq!(remaining_loan.borrowed_balance, expected_remaining_borrowed);

			// Ensure that the remaining collateral in the loan is correct
			assert_eq!(
				remaining_loan.collateral_balance,
				ksm_collateral_amount - expected_collateral_release
			);

			// Verify that pallet's DOT balance increased by repayment amount
			assert_eq!(
				get_pallet_balance(DOT),
				pallet_initial_dot_balance - dot_borrow_amount + repayment_amount
			);

			// Verify that pallet's KSM balance decreased by released collateral
			assert_eq!(
				get_pallet_balance(KSM),
				pallet_initial_ksm_balance + ksm_collateral_amount - expected_collateral_release
			);
		});
}

#[test]
fn repay_full_with_accumulated_interest() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, BOB, 1_000_000),
			(DOT, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 100_000); // Provide sufficient initial liquidity

			// Record initial balances
			let bob_initial_ksm_balance = Fungibles::balance(KSM, &BOB);
			let bob_initial_dot_balance = Fungibles::balance(DOT, &BOB);
			let pallet_initial_dot_balance = get_pallet_balance(DOT);
			let pallet_initial_ksm_balance = get_pallet_balance(KSM);

			// Verify that the DOT pool exists
			let asset_pool = AssetPool::<Test>::from(DOT);
			assert!(Lending::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			// Set price: 1 DOT = 1 KSM for simplicity
			let price = FixedU128::from_rational(1, 1);
			assert_ok!(Lending::set_asset_price(RuntimeOrigin::signed(ALICE), DOT, KSM, price));

			let ksm_collateral_amount = 100_000;
			let dot_borrow_amount = 50_000;
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,               // asset to borrow
				dot_borrow_amount, // amount to borrow
				KSM
			));

			// Verify balances after borrowing
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount
			);
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance + dot_borrow_amount);

			// Simulate time passing by advancing the timestamp
			let current_block_number = System::block_number();
			let blocks_per_month = 30 * 24 * 60 * 60 / 6;
			let new_block_number = current_block_number + blocks_per_month;

			// Advance to the new block number
			run_to_block(new_block_number);

			// Now, the interest should have accumulated over one month

			// Fetch the lending pool to get the updated borrow index
			let asset_pool = AssetPool::<Test>::from(DOT);
			let mut pool = Lending::reserve_pools(asset_pool.clone()).unwrap();

			// Update the pool's indexes to accrue interest
			assert_ok!(pool.update_indexes());

			// Update the storage with the new pool state
			Lending::reserve_pools(asset_pool);

			// Get BOB's loan
			let loan = Borrows::<Test>::get((BOB, DOT, KSM)).expect("Loan should exist");

			// Calculate the repayable amount
			let repayable_amount = pool.repayable_amount(&loan).unwrap();

			// Assert that the repayable amount is greater than the initial borrowed amount
			assert!(repayable_amount > dot_borrow_amount);

			// BOB needs to repay the repayable_amount
			// Ensure BOB has enough balance to repay
			let bob_dot_balance = Fungibles::balance(DOT, &BOB);
			assert!(bob_dot_balance >= repayable_amount);

			// Repay the loan
			assert_ok!(Lending::repay(RuntimeOrigin::signed(BOB), DOT, repayable_amount, KSM));

			// Check if the repay event was emitted
			System::assert_last_event(
				Event::Repaid {
					who: BOB,
					repaid_asset_id: DOT,
					repaid_balance: repayable_amount,
					collateral_asset_id: KSM,
					collateral_balance: ksm_collateral_amount,
				}
				.into(),
			);

			// Verify final balances
			// BOB's KSM balance should be back to initial (collateral returned)
			assert_eq!(Fungibles::balance(KSM, &BOB), bob_initial_ksm_balance);

			// BOB's DOT balance should have decreased by repayable_amount
			assert_eq!(
				Fungibles::balance(DOT, &BOB),
				bob_initial_dot_balance + dot_borrow_amount - repayable_amount
			);

			// The difference should be the interest
			let interest_paid = repayable_amount - dot_borrow_amount;
			assert!(interest_paid > 0);

			// Pallet's DOT balance should have increased by repayable_amount
			assert_eq!(
				get_pallet_balance(DOT),
				pallet_initial_dot_balance - dot_borrow_amount + repayable_amount
			);

			// Pallet's KSM balance should be back to initial (collateral returned)
			assert_eq!(get_pallet_balance(KSM), pallet_initial_ksm_balance);
		});
}

#[test]
fn partial_repay_with_accumulated_interest() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, BOB, 1_000_000),
			(DOT, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			setup_active_pool(DOT, 100_000);

			let bob_initial_ksm_balance = Fungibles::balance(KSM, &BOB);
			let bob_initial_dot_balance = Fungibles::balance(DOT, &BOB);
			let asset_pool = AssetPool::<Test>::from(DOT);
			assert!(Lending::reserve_pools(asset_pool.clone()).is_some(), "DOT pool should exist");
			let price = FixedU128::from_rational(1, 1);
			assert_ok!(Lending::set_asset_price(RuntimeOrigin::signed(ALICE), DOT, KSM, price));

			// Bob borrows DOT by providing KSM as collateral
			let ksm_collateral_amount = 100_000;
			let dot_borrow_amount = 50_000;
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,               // asset to borrow
				dot_borrow_amount, // amount to borrow
				KSM
			));

			// Verify balances after borrowing
			assert_eq!(
				Fungibles::balance(KSM, &BOB),
				bob_initial_ksm_balance - ksm_collateral_amount
			);
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_initial_dot_balance + dot_borrow_amount);

			const SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60;
			const BLOCK_TIME: u64 = 6; // 6 seconds per block

			let current_block_number = System::block_number();
			let blocks_per_year = SECONDS_PER_YEAR / BLOCK_TIME;
			let new_block_number = current_block_number + blocks_per_year / 2; // Advance half a year

			run_to_block(new_block_number);

			// The interest should have accumulated over half a year
			let mut pool = Lending::reserve_pools(asset_pool.clone()).unwrap();
			// Update the pool's indexes to accrue interest
			assert_ok!(pool.update_indexes());

			// Get BOB's loan
			let mut loan = Borrows::<Test>::get((BOB, DOT, KSM)).expect("Loan should exist");

			// Calculate the repayable amount
			let repayable_amount = pool.repayable_amount(&loan).unwrap();

			println!("Total repayable amount: {:?}", repayable_amount);
			let partial_repayment = repayable_amount / 2;
			let bob_dot_balance = Fungibles::balance(DOT, &BOB);
			assert!(bob_dot_balance >= partial_repayment);

			// Record balances before repayment
			let bob_dot_balance_before = bob_dot_balance;
			let bob_ksm_balance_before = Fungibles::balance(KSM, &BOB);

			// Bob makes a partial repayment
			assert_ok!(Lending::repay(RuntimeOrigin::signed(BOB), DOT, partial_repayment, KSM));

			// Fetch the updated loan
			loan = Borrows::<Test>::get((BOB, DOT, KSM)).expect("Loan should still exist");

			// Verify Loan State After Partial Repayment
			// Calculate expected remaining borrowed balance
			let expected_remaining_borrowed_balance = loan.borrowed_balance;

			// Ensure that the borrowed balance has decreased
			println!(
				"Remaining borrowed balance (scaled): {:?}",
				expected_remaining_borrowed_balance
			);

			// Calculate the remaining repayable amount
			let remaining_repayable_amount = pool.repayable_amount(&loan).unwrap();

			println!(
				"Remaining repayable amount after partial repayment: {:?}",
				remaining_repayable_amount
			);

			// Check that the remaining repayable amount is approximately half of the original
			assert!(
				remaining_repayable_amount > repayable_amount / 2 - 1_000 &&
					remaining_repayable_amount < repayable_amount / 2 + 1_000,
				"Remaining repayable amount should be approximately half of the original"
			);

			// Verify that Bob's DOT balance has decreased by the partial repayment amount
			assert_eq!(Fungibles::balance(DOT, &BOB), bob_dot_balance_before - partial_repayment);

			// Verify that Bob's KSM balance has increased by the released collateral
			let bob_ksm_balance_after = Fungibles::balance(KSM, &BOB);
			assert!(bob_ksm_balance_after > bob_ksm_balance_before);

			let released_collateral = bob_ksm_balance_after - bob_ksm_balance_before;
			println!("Released collateral amount: {:?}", released_collateral);

			// Verify that some collateral has been released
			assert!(released_collateral > 0);

			// Verify that the pool's borrowed balance has decreased by the principal reduction
			let expected_principal_reduction =
				partial_repayment - (partial_repayment * 1_000_000 / 1_030_457);

			println!("Expected principal reduction: {:?}", expected_principal_reduction);

			// Fetch the updated pool
			pool = Lending::reserve_pools(asset_pool.clone()).unwrap();

			// Verify that the pool's borrowed balance has decreased
			println!("Pool's borrowed balance after repayment: {:?}", pool.borrowed_balance);
		});
}
