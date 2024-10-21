use crate::{tests::mock::*, AssetPool};
use frame_support::assert_ok;
use num_traits::Zero;
use sp_runtime::{FixedPointNumber, FixedU128};

#[test]
fn test_compute_user_ltv_on_max_borrow() {
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

			// Verify that the DOT pool exists
			let asset_pool = AssetPool::<Test>::from(DOT);
			assert!(Lending::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			let price = FixedU128::from_rational(1, 2);
			let dot_borrow_amount = 500;

			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1), // 1 KSM = 2 USDT
			));

			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				KSM,
				price
			));

			assert_ok!(Lending::supply(RuntimeOrigin::signed(BOB), DOT, 1_000));
			let ksm_collateral_amount =
				Lending::estimate_collateral_amount(DOT, dot_borrow_amount, KSM).unwrap();
			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,               // asset to borrow
				dot_borrow_amount, // amount to borrow
				KSM
			));

			let (current_ltv, sale_ltv, liq_ltv) = Lending::compute_user_ltv(&BOB);

			let expected_current_ltv =
				FixedU128::saturating_from_rational(dot_borrow_amount, ksm_collateral_amount * 2);
			let expected_sale_ltv = FixedU128::saturating_from_rational(50_000u128, 100_000u128);
			let expected_liq_ltv = FixedU128::saturating_from_rational(80_000u128, 100_000u128);

			assert_eq!(current_ltv, expected_current_ltv);
			assert_eq!(sale_ltv, expected_sale_ltv);
			assert_eq!(liq_ltv, expected_liq_ltv);
		});
}

#[test]
fn test_get_asset_price_with_usdt() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Lending::set_asset_price(
			RuntimeOrigin::signed(ALICE),
			DOT,
			USDT,
			FixedU128::from_rational(10, 1), // 1 DOT = 10 USDT
		));

		let price = Lending::get_asset_price(DOT, 1);
		assert_ok!(&price);

		let expected_price = FixedU128::from_u32(10);
		assert_eq!(price.unwrap(), expected_price);
	});
}

#[test]
fn test_get_asset_price_with_base() {
	ExtBuilder::default().build().execute_with(|| {
		// Set DOT price in terms of USDT: 1 DOT = 10 USDT
		assert_ok!(Lending::set_asset_price(
			RuntimeOrigin::signed(ALICE),
			DOT,
			USDT,
			FixedU128::from_rational(10, 1),
		));

		// Set KSM price in terms of USDT: 1 KSM = 20 USDT
		assert_ok!(Lending::set_asset_price(
			RuntimeOrigin::signed(ALICE),
			KSM,
			USDT,
			FixedU128::from_rational(20, 1),
		));

		let price = Lending::get_asset_price(DOT, KSM);
		assert_ok!(&price);

		let expected_price = FixedU128::from_rational(1, 2);
		assert_eq!(price.unwrap(), expected_price);
	});
}

#[test]
fn test_get_asset_price_with_error() {
	ExtBuilder::default().build().execute_with(|| {
		let err_price = Lending::get_asset_price(DOT, KSM);
		assert!(matches!(err_price, Err(_)));
	});
}

#[test]
fn test_get_asset_wise_supplies_with_one_supply() {
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

			let price = FixedU128::from_rational(1, 2);
			let ksm_collateral_amount = 10_000;
			let dot_borrow_amount = 500;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1), // 1 KSM = 2 USDT
			));

			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				KSM,
				price
			));

			assert_ok!(Lending::supply(RuntimeOrigin::signed(BOB), DOT, 1_000));

			let (supplied_assets, total_supply) = Lending::get_asset_wise_supplies(&BOB);
			let supplied_assets_size = supplied_assets.len();
			assert_eq!(supplied_assets_size, 1);

			let supplied_asset = supplied_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(supplied_asset.asset_info.asset_id, DOT);
			assert_eq!(supplied_asset.asset_info.asset_name, expected_name);
			assert_eq!(supplied_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(supplied_asset.asset_info.decimals, expected_decimals);
			assert_eq!(supplied_asset.asset_info.balance, 1_000_000 - 1_000);
			assert_eq!(supplied_asset.apy, FixedU128::zero());
			assert_eq!(supplied_asset.supplied, 1_000);
			assert_eq!(total_supply, 1_000);

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			// apy and balance should be changed
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,               // asset to borrow
				dot_borrow_amount, // amount to borrow
				KSM
			));

			let (supplied_assets, total_supply) = Lending::get_asset_wise_supplies(&BOB);
			let supplied_assets_size = supplied_assets.len();
			assert_eq!(supplied_assets_size, 1);

			let supplied_asset = supplied_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(supplied_asset.asset_info.asset_id, DOT);
			assert_eq!(supplied_asset.asset_info.asset_name, expected_name);
			assert_eq!(supplied_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(supplied_asset.asset_info.decimals, expected_decimals);
			assert_eq!(supplied_asset.asset_info.balance, 1_000_000 - 1_000 + dot_borrow_amount);
			assert_ne!(supplied_asset.apy, FixedU128::zero());
			assert_eq!(supplied_asset.supplied, 1_000);
			assert_eq!(total_supply, 1_000);
		});
}

#[test]
fn test_get_asset_wise_supplies_with_two_supply() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 1000);
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 1,
				KSM,
				1000
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), KSM));

			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1),
			));

			assert_ok!(Lending::supply(RuntimeOrigin::signed(BOB), DOT, 1_000));
			assert_ok!(Lending::supply(RuntimeOrigin::signed(BOB), KSM, 500));

			let (supplied_assets, total_supply) = Lending::get_asset_wise_supplies(&BOB);
			let supplied_assets_size = supplied_assets.len();
			assert_eq!(supplied_assets_size, 2);

			// DOT
			let dot_supplied_asset = supplied_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(dot_supplied_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_supplied_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_supplied_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_supplied_asset.asset_info.decimals, expected_decimals);
			assert_eq!(dot_supplied_asset.asset_info.balance, 1_000_000 - 1_000);
			assert_eq!(dot_supplied_asset.apy, FixedU128::zero());
			assert_eq!(dot_supplied_asset.supplied, 1_000);

			//KSM
			let ksm_supplied_asset = supplied_assets.last().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(KSM);
			assert_eq!(ksm_supplied_asset.asset_info.asset_id, KSM);
			assert_eq!(ksm_supplied_asset.asset_info.asset_name, expected_name);
			assert_eq!(ksm_supplied_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(ksm_supplied_asset.asset_info.decimals, expected_decimals);
			assert_eq!(ksm_supplied_asset.asset_info.balance, 1_000_000 - 500);
			assert_eq!(ksm_supplied_asset.apy, FixedU128::zero());
			assert_eq!(ksm_supplied_asset.supplied, 500);

			// total in USDT
			assert_eq!(total_supply, 2_000);
		});
}

#[test]
fn test_get_asset_wise_borrows_collaterals_with_one_borrow() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 1000);

			let dot_borrow_amount = 500;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1),
			));

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,               // asset to borrow
				dot_borrow_amount, // amount to borrow
				KSM
			));

			let (borrowed_assets, collateral_assets, total_borrow, total_collateral) =
				Lending::get_asset_wise_borrows_collaterals(&BOB);
			let ksm_collateral_amount =
				Lending::estimate_collateral_amount(DOT, dot_borrow_amount, KSM).unwrap();
			assert_eq!(borrowed_assets.len(), 1);
			assert_eq!(collateral_assets.len(), 1);

			// DOT
			let dot_borrowed_asset = borrowed_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_borrowed_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_borrowed_asset.asset_info.decimals, expected_decimals);
			assert_eq!(dot_borrowed_asset.asset_info.balance, 1_000_000 + dot_borrow_amount);
			assert_ne!(dot_borrowed_asset.apy, FixedU128::zero());
			assert_eq!(dot_borrowed_asset.borrowed, dot_borrow_amount);

			//KSM
			let ksm_collateral_asset = collateral_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(KSM);
			assert_eq!(ksm_collateral_asset.asset_info.asset_id, KSM);
			assert_eq!(ksm_collateral_asset.asset_info.asset_name, expected_name);
			assert_eq!(ksm_collateral_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(ksm_collateral_asset.asset_info.decimals, expected_decimals);
			assert_eq!(ksm_collateral_asset.asset_info.balance, ksm_collateral_amount);

			// total in USDT
			assert_eq!(total_borrow, dot_borrowed_asset.borrowed);
			assert_eq!(total_collateral, ksm_collateral_asset.asset_info.balance * 2);
		});
}

#[test]
fn test_get_asset_wise_borrows_collaterals_with_two_borrows() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 1000);
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 1,
				KSM,
				1000
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), KSM));

			let dot_borrow_amount_1 = 500;
			let ksm_borrow_amount_2 = 1000;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1),
			));

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                 // asset to borrow
				dot_borrow_amount_1, // amount to borrow
				KSM
			));

			// BOB borrows 1000 KSM using 5_000 DOT as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				KSM,                 // asset to borrow
				ksm_borrow_amount_2, // amount to borrow
				DOT
			));

			let (borrowed_assets, collateral_assets, total_borrow, total_collateral) =
				Lending::get_asset_wise_borrows_collaterals(&BOB);

			assert_eq!(borrowed_assets.len(), 2);
			assert_eq!(collateral_assets.len(), 2);
			let ksm_collateral_amount_1 =
				Lending::estimate_collateral_amount(DOT, dot_borrow_amount_1, KSM).unwrap();
			let dot_collateral_amount_2 =
				Lending::estimate_collateral_amount(KSM, ksm_borrow_amount_2, DOT).unwrap();
			//First borrow
			// DOT
			let dot_borrowed_asset = borrowed_assets.last().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_borrowed_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_borrowed_asset.asset_info.decimals, expected_decimals);
			assert_eq!(
				dot_borrowed_asset.asset_info.balance,
				1_000_000 + dot_borrow_amount_1 - dot_collateral_amount_2
			);
			assert_ne!(dot_borrowed_asset.apy, FixedU128::zero());
			assert_eq!(dot_borrowed_asset.borrowed, dot_borrow_amount_1);
			//KSM
			let ksm_collateral_asset = collateral_assets.last().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(KSM);
			assert_eq!(ksm_collateral_asset.asset_info.asset_id, KSM);
			assert_eq!(ksm_collateral_asset.asset_info.asset_name, expected_name);
			assert_eq!(ksm_collateral_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(ksm_collateral_asset.asset_info.decimals, expected_decimals);
			assert_eq!(ksm_collateral_asset.asset_info.balance, ksm_collateral_amount_1);

			//Second borrow
			//KSM
			let ksm_borrowed_asset = borrowed_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(KSM);
			assert_eq!(ksm_borrowed_asset.asset_info.asset_id, KSM);
			assert_eq!(ksm_borrowed_asset.asset_info.asset_name, expected_name);
			assert_eq!(ksm_borrowed_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(ksm_borrowed_asset.asset_info.decimals, expected_decimals);
			assert_eq!(
				ksm_borrowed_asset.asset_info.balance,
				1_000_000 + ksm_borrow_amount_2 - ksm_collateral_amount_1
			);
			assert_eq!(ksm_borrowed_asset.apy, FixedU128::zero());
			assert_eq!(ksm_borrowed_asset.borrowed, ksm_borrow_amount_2);
			//DOT
			let dot_collateral_asset = collateral_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(dot_collateral_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_collateral_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_collateral_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_collateral_asset.asset_info.decimals, expected_decimals);
			assert_eq!(dot_collateral_asset.asset_info.balance, dot_collateral_amount_2);

			// total in USDT
			assert_eq!(total_borrow, dot_borrowed_asset.borrowed + ksm_borrowed_asset.borrowed * 2);
			assert_eq!(
				total_collateral,
				ksm_collateral_asset.asset_info.balance * 2
					+ dot_collateral_asset.asset_info.balance
			);
		});
}

#[test]
fn test_get_lending_pools_without_params() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			// setup_active_pool(DOT, 1000);
			let initial_balance = 1000;
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 2,
				DOT,
				initial_balance
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT));

			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 1,
				KSM,
				initial_balance
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), KSM));

			let ksm_collateral_amount_1 = 10_000;
			let dot_borrow_amount_1 = 500;
			let dot_collateral_amount_2 = 5000;
			let ksm_borrow_amount_2 = 1000;
			let ksm_supplied = 1000;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1),
			));

			assert_ok!(Lending::supply(RuntimeOrigin::signed(ALICE), KSM, ksm_supplied));

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                 // asset to borrow
				dot_borrow_amount_1, // amount to borrow
				KSM
			));

			// BOB borrows 1000 KSM using 5_000 DOT as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(ALICE),
				KSM,                 // asset to borrow
				ksm_borrow_amount_2, // amount to borrow
				DOT
			));

			// case without parameters, all lending pools
			let (lending_pools, totals) = Lending::get_lending_pools(None, None);

			assert_eq!(lending_pools.len(), 2);

			// DOT
			let dot_lending_pool = lending_pools.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(dot_lending_pool.asset_id, DOT);
			assert_eq!(dot_lending_pool.asset, expected_name);
			assert_eq!(dot_lending_pool.asset_symbol, expected_symbol);
			assert_eq!(dot_lending_pool.asset_decimals, expected_decimals);
			assert_ne!(dot_lending_pool.collateral_q, 0u64);
			assert_ne!(dot_lending_pool.utilization, FixedU128::zero());
			assert_ne!(dot_lending_pool.borrow_apy, FixedU128::zero());
			assert_ne!(dot_lending_pool.supply_apy, FixedU128::zero());
			assert_eq!(dot_lending_pool.user_supplied_balance, None);
			assert_eq!(dot_lending_pool.user_asset_balance, None);

			//KSM
			let ksm_lending_pool = lending_pools.last().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(KSM);
			assert_eq!(ksm_lending_pool.asset_id, KSM);
			assert_eq!(ksm_lending_pool.asset, expected_name);
			assert_eq!(ksm_lending_pool.asset_symbol, expected_symbol);
			assert_eq!(ksm_lending_pool.asset_decimals, expected_decimals);
			assert_ne!(ksm_lending_pool.collateral_q, 0u64);
			assert_ne!(ksm_lending_pool.utilization, FixedU128::zero());
			assert_ne!(ksm_lending_pool.borrow_apy, FixedU128::zero());
			assert_ne!(ksm_lending_pool.supply_apy, FixedU128::zero());
			assert_eq!(dot_lending_pool.user_supplied_balance, None);
			assert_eq!(ksm_lending_pool.user_asset_balance, None);

			// total in USDT supply = initial balance of pools 3000 - borrow amounts 2500 + supplied
			// 2000
			assert_eq!(
				totals.total_supply,
				initial_balance * 2 + initial_balance
					- dot_borrow_amount_1
					- ksm_borrow_amount_2 * 2
					+ ksm_supplied * 2
			);
			assert_eq!(totals.total_borrow, ksm_borrow_amount_2 * 2 + dot_borrow_amount_1);
		});
}

#[test]
fn test_get_lending_pools_with_asset_param() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			// setup_active_pool(DOT, 1000);
			let initial_balance = 1000;
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 2,
				DOT,
				initial_balance
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT));

			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 1,
				KSM,
				initial_balance
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), KSM));

			let ksm_collateral_amount_1 = 10_000;
			let dot_borrow_amount_1 = 500;
			let dot_collateral_amount_2 = 5000;
			let ksm_borrow_amount_2 = 1000;
			let ksm_supplied = 1000;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1),
			));

			assert_ok!(Lending::supply(RuntimeOrigin::signed(ALICE), KSM, ksm_supplied));

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                 // asset to borrow
				dot_borrow_amount_1, // amount to borrow
				KSM
			));

			// BOB borrows 1000 KSM using 5_000 DOT as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(ALICE),
				KSM,                 // asset to borrow
				ksm_borrow_amount_2, // amount to borrow
				DOT
			));

			// case with asset id
			let (lending_pools, totals) = Lending::get_lending_pools(Some(DOT), None);

			assert_eq!(lending_pools.len(), 1);

			// DOT
			let dot_lending_pool = lending_pools.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(dot_lending_pool.asset_id, DOT);
			assert_eq!(dot_lending_pool.asset, expected_name);
			assert_eq!(dot_lending_pool.asset_symbol, expected_symbol);
			assert_eq!(dot_lending_pool.asset_decimals, expected_decimals);
			assert_ne!(dot_lending_pool.collateral_q, 0u64);
			assert_ne!(dot_lending_pool.utilization, FixedU128::zero());
			assert_ne!(dot_lending_pool.borrow_apy, FixedU128::zero());
			assert_ne!(dot_lending_pool.supply_apy, FixedU128::zero());
			assert_eq!(dot_lending_pool.user_supplied_balance, None);
			assert_eq!(dot_lending_pool.user_asset_balance, None);
			assert_eq!(totals.total_supply, initial_balance - dot_borrow_amount_1);
			assert_eq!(totals.total_borrow, dot_borrow_amount_1);
		});
}

#[test]
fn test_get_lending_pools_with_account_and_asset() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
			(KSM, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			// setup_active_pool(DOT, 4333);
			let initial_balance = 4333;
			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 2,
				DOT,
				initial_balance
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), DOT));

			assert_ok!(Lending::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN + 1,
				KSM,
				initial_balance
			));
			assert_ok!(Lending::activate_lending_pool(RuntimeOrigin::signed(ALICE), KSM));

			let dot_borrow_amount_1 = 500;
			let ksm_borrow_amount_2 = 1000;
			let ksm_supplied = 1234;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 1 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(1, 1),
			));

			assert_ok!(Lending::supply(RuntimeOrigin::signed(ALICE), KSM, ksm_supplied));

			// BOB borrows 500 DOT using KSM as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                 // asset to borrow
				dot_borrow_amount_1, // amount to borrow
				KSM
			));

			// ALICE borrows 1000 KSM using DOT as collateral
			assert_ok!(Lending::borrow(
				RuntimeOrigin::signed(ALICE),
				KSM,                 // asset to borrow
				ksm_borrow_amount_2, // amount to borrow
				DOT
			));

			// case with asset id
			let (lending_pools, totals) = Lending::get_lending_pools(Some(DOT), Some(&BOB));
			assert_eq!(lending_pools.len(), 1);
			// DOT
			let dot_lending_pool = lending_pools.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(DOT);
			assert_eq!(dot_lending_pool.asset_id, DOT);
			assert_eq!(dot_lending_pool.asset, expected_name);
			assert_eq!(dot_lending_pool.asset_symbol, expected_symbol);
			assert_eq!(dot_lending_pool.asset_decimals, expected_decimals);
			assert_ne!(dot_lending_pool.collateral_q, 0u64);
			assert_ne!(dot_lending_pool.utilization, FixedU128::zero());
			assert_ne!(dot_lending_pool.borrow_apy, FixedU128::zero());
			assert_ne!(dot_lending_pool.supply_apy, FixedU128::zero());
			assert_eq!(dot_lending_pool.user_supplied_balance, Some(0));
			assert_eq!(dot_lending_pool.user_asset_balance, Some(1_000_000 + dot_borrow_amount_1));
			assert_eq!(totals.total_supply, initial_balance - dot_borrow_amount_1);
			assert_eq!(totals.total_borrow, dot_borrow_amount_1);

			let (lending_pools, totals) = Lending::get_lending_pools(Some(KSM), Some(&ALICE));
			assert_eq!(lending_pools.len(), 1);
			// KSM
			let ksm_lending_pool = lending_pools.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) =
				Lending::get_metadata(KSM);
			assert_eq!(ksm_lending_pool.asset_id, KSM);
			assert_eq!(ksm_lending_pool.asset, expected_name);
			assert_eq!(ksm_lending_pool.asset_symbol, expected_symbol);
			assert_eq!(ksm_lending_pool.asset_decimals, expected_decimals);
			assert_ne!(ksm_lending_pool.collateral_q, 0u64);
			assert_ne!(ksm_lending_pool.utilization, FixedU128::zero());
			assert_ne!(ksm_lending_pool.borrow_apy, FixedU128::zero());
			assert_ne!(ksm_lending_pool.supply_apy, FixedU128::zero());
			assert_eq!(ksm_lending_pool.user_supplied_balance, Some(initial_balance + ksm_supplied));
			assert_eq!(ksm_lending_pool.user_asset_balance, Some(1_000_000 + ksm_borrow_amount_2 - ksm_supplied - initial_balance));
			assert_eq!(totals.total_supply, initial_balance + ksm_supplied - ksm_borrow_amount_2);
			assert_eq!(totals.total_borrow, ksm_borrow_amount_2);

		});
}

#[test]
fn test_get_estimate_collateral_amount() {
	ExtBuilder::default()
		.with_endowed_balances(vec![
			(DOT, ALICE, 1_000_000),
			(KSM, ALICE, 1_000_000),
			(DOT, BOB, 1_000_000),
		])
		.build()
		.execute_with(|| {
			// Setup and activate the DOT lending pool
			setup_active_pool(DOT, 1000);

			// Set DOT price in terms of USDT: 1 DOT = 10 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(10, 1),
			));

			// Set KSM price in terms of USDT: 1 KSM = 5 USDT
			assert_ok!(Lending::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(5, 1),
			));
			let estimate_collateral_amount =
				Lending::estimate_collateral_amount(DOT, 100, KSM);

			assert_ok!(&estimate_collateral_amount);

			//Default collateral factor at 50%
			//collateral_amount = borrow_amount/collateral_factor = 100/0.5 = 200 DOT = 400 KSM
			let expected_amount = 400;
			assert_eq!(estimate_collateral_amount.unwrap(), expected_amount);
		});
}

#[test]
fn test_get_estimate_collateral_amount_with_error() {
	ExtBuilder::default().build().execute_with(|| {
		let err_amount = Lending::estimate_collateral_amount(DOT, 100, KSM);
		assert!(matches!(err_amount, Err(_)));
	});
}
