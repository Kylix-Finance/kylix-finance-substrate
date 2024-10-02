use crate::{tests::mock::*, AssetPool};
use frame_support::{assert_err, assert_ok};
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
			assert!(TemplateModule::reserve_pools(asset_pool).is_some(), "DOT pool should exist");

			let price = FixedU128::from_rational(1, 2);
			let ksm_collateral_amount = 10_000;
			let dot_borrow_amount = 500;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1), // 1 KSM = 2 USDT
			));

			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				KSM,
				price
			));

			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 1_000));

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                   // asset to borrow
				dot_borrow_amount,     // amount to borrow
				KSM,                   // collateral asset
				ksm_collateral_amount  // collateral amount
			));

			let (current_ltv, sale_ltv, liq_ltv) = TemplateModule::compute_user_ltv(&BOB);

			let expected_current_ltv = FixedU128::saturating_from_rational(10_000u128, 100_000u128);
			let expected_sale_ltv = FixedU128::saturating_from_rational(50_000u128, 100_000u128);
			let expected_liq_ltv = FixedU128::saturating_from_rational(80_000u128, 100_000u128);

			assert_eq!(current_ltv, expected_current_ltv);
			assert_eq!(sale_ltv, expected_sale_ltv);
			assert_eq!(liq_ltv, expected_liq_ltv);
		});
}

#[test]
fn test_get_asset_price_with_usdt() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(10, 1), // 1 DOT = 10 USDT
			));

			let price = TemplateModule::get_asset_price(DOT, 1);
			assert_ok!(&price);

			let expected_price = FixedU128::from_u32(10);
			assert_eq!(price.unwrap(), expected_price);
		});
}

#[test]
fn test_get_asset_price_with_base() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			// Set DOT price in terms of USDT: 1 DOT = 10 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(10, 1), 
			));

			// Set KSM price in terms of USDT: 1 KSM = 20 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(20, 1), 
			));

			let price = TemplateModule::get_asset_price(DOT, KSM);
			assert_ok!(&price);

			let expected_price = FixedU128::from_rational(1, 2);
			assert_eq!(price.unwrap(), expected_price);
		});
}

#[test]
fn test_get_asset_price_with_error() {
	ExtBuilder::default()
		.build()
		.execute_with(|| {
			let err_price = TemplateModule::get_asset_price(DOT, KSM);
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
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1), // 1 KSM = 2 USDT
			));

			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				KSM,
				price
			));

			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 1_000));
			
			let (supplied_assets, total_supply) = TemplateModule::get_asset_wise_supplies(&BOB);
			let supplied_assets_size = supplied_assets.len();
			assert_eq!(supplied_assets_size, 1);

			let supplied_asset = supplied_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(DOT);
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
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                   // asset to borrow
				dot_borrow_amount,     // amount to borrow
				KSM,                   // collateral asset
				ksm_collateral_amount  // collateral amount
			));

			let (supplied_assets, total_supply) = TemplateModule::get_asset_wise_supplies(&BOB);
			let supplied_assets_size = supplied_assets.len();
			assert_eq!(supplied_assets_size, 1);

			let supplied_asset = supplied_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(DOT);
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
			assert_ok!(TemplateModule::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN+1,
				KSM,
				1000
			));
			assert_ok!(TemplateModule::activate_lending_pool(RuntimeOrigin::signed(ALICE), KSM));

			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1), 
			));

			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), DOT, 1_000));
			assert_ok!(TemplateModule::supply(RuntimeOrigin::signed(BOB), KSM, 500));

			let (supplied_assets, total_supply) = TemplateModule::get_asset_wise_supplies(&BOB);
			let supplied_assets_size = supplied_assets.len();
			assert_eq!(supplied_assets_size, 2);

			// DOT
			let dot_supplied_asset = supplied_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(DOT);
			assert_eq!(dot_supplied_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_supplied_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_supplied_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_supplied_asset.asset_info.decimals, expected_decimals);
			assert_eq!(dot_supplied_asset.asset_info.balance, 1_000_000 - 1_000);
			assert_eq!(dot_supplied_asset.apy, FixedU128::zero());
			assert_eq!(dot_supplied_asset.supplied, 1_000);

			//KSM
			let ksm_supplied_asset = supplied_assets.last().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(KSM);
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
			
			let ksm_collateral_amount = 10_000;
			let dot_borrow_amount = 500;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1), 
			));

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                   // asset to borrow
				dot_borrow_amount,     // amount to borrow
				KSM,                   // collateral asset
				ksm_collateral_amount  // collateral amount
			));

			let (borrowed_assets, collateral_assets, total_borrow, total_collateral) = TemplateModule::get_asset_wise_borrows_collaterals(&BOB);
			
			assert_eq!(borrowed_assets.len(), 1);
			assert_eq!(collateral_assets.len(), 1);

			// DOT
			let dot_borrowed_asset = borrowed_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_borrowed_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_borrowed_asset.asset_info.decimals, expected_decimals);
			assert_eq!(dot_borrowed_asset.asset_info.balance, 1_000_000 + dot_borrow_amount);
			assert_ne!(dot_borrowed_asset.apy, FixedU128::zero());
			assert_eq!(dot_borrowed_asset.borrowed, dot_borrow_amount);

			//KSM
			let ksm_collateral_asset = collateral_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(KSM);
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
			assert_ok!(TemplateModule::create_lending_pool(
				RuntimeOrigin::signed(ALICE),
				LENDING_POOL_TOKEN+1,
				KSM,
				1000
			));
			assert_ok!(TemplateModule::activate_lending_pool(RuntimeOrigin::signed(ALICE), KSM));

			let ksm_collateral_amount_1 = 10_000;
			let dot_borrow_amount_1 = 500;
			let dot_collateral_amount_2 = 5000;
			let ksm_borrow_amount_2 = 1000;
			// Set DOT price in terms of USDT: 1 DOT = 1 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				USDT,
				FixedU128::from_rational(1, 1), // 1 DOT = 1 USDT
			));

			// Set KSM price in terms of USDT: 1 KSM = 2 USDT
			assert_ok!(TemplateModule::set_asset_price(
				RuntimeOrigin::signed(ALICE),
				KSM,
				USDT,
				FixedU128::from_rational(2, 1), 
			));

			// BOB borrows 500 DOT using 10_000 KSM as collateral
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				DOT,                   // asset to borrow
				dot_borrow_amount_1,     // amount to borrow
				KSM,                   // collateral asset
				ksm_collateral_amount_1  // collateral amount
			));

			// BOB borrows 1000 KSM using 5_000 DOT as collateral
			assert_ok!(TemplateModule::borrow(
				RuntimeOrigin::signed(BOB),
				KSM,                   // asset to borrow
				ksm_borrow_amount_2,     // amount to borrow
				DOT,                   // collateral asset
				dot_collateral_amount_2  // collateral amount
			));

			let (borrowed_assets, collateral_assets, total_borrow, total_collateral) = TemplateModule::get_asset_wise_borrows_collaterals(&BOB);
			
			assert_eq!(borrowed_assets.len(), 2);
			assert_eq!(collateral_assets.len(), 2);

			//First borrow
			// DOT
			let dot_borrowed_asset = borrowed_assets.last().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_borrowed_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_borrowed_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_borrowed_asset.asset_info.decimals, expected_decimals);
			assert_eq!(dot_borrowed_asset.asset_info.balance, 1_000_000 + dot_borrow_amount_1 - dot_collateral_amount_2);
			assert_ne!(dot_borrowed_asset.apy, FixedU128::zero());
			assert_eq!(dot_borrowed_asset.borrowed, dot_borrow_amount_1);
			//KSM
			let ksm_collateral_asset = collateral_assets.last().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(KSM);
			assert_eq!(ksm_collateral_asset.asset_info.asset_id, KSM);
			assert_eq!(ksm_collateral_asset.asset_info.asset_name, expected_name);
			assert_eq!(ksm_collateral_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(ksm_collateral_asset.asset_info.decimals, expected_decimals);
			assert_eq!(ksm_collateral_asset.asset_info.balance, ksm_collateral_amount_1);

			//Second borrow
			//KSM
			let ksm_borrowed_asset = borrowed_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(KSM);
			assert_eq!(ksm_borrowed_asset.asset_info.asset_id, KSM);
			assert_eq!(ksm_borrowed_asset.asset_info.asset_name, expected_name);
			assert_eq!(ksm_borrowed_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(ksm_borrowed_asset.asset_info.decimals, expected_decimals);
			assert_eq!(ksm_borrowed_asset.asset_info.balance, 1_000_000 + ksm_borrow_amount_2 - ksm_collateral_amount_1);
			assert_eq!(ksm_borrowed_asset.apy, FixedU128::zero());
			assert_eq!(ksm_borrowed_asset.borrowed, ksm_borrow_amount_2);
			//DOT
			let dot_collateral_asset = collateral_assets.first().unwrap();
			let (expected_name, expected_decimals, expected_symbol) = TemplateModule::get_metadata(DOT);
			assert_eq!(dot_collateral_asset.asset_info.asset_id, DOT);
			assert_eq!(dot_collateral_asset.asset_info.asset_name, expected_name);
			assert_eq!(dot_collateral_asset.asset_info.asset_symbol, expected_symbol);
			assert_eq!(dot_collateral_asset.asset_info.decimals, expected_decimals);
			assert_eq!(dot_collateral_asset.asset_info.balance, dot_collateral_amount_2);

			// total in USDT
			assert_eq!(total_borrow, dot_borrowed_asset.borrowed + ksm_borrowed_asset.borrowed * 2);
			assert_eq!(total_collateral, ksm_collateral_asset.asset_info.balance * 2 + dot_collateral_asset.asset_info.balance);
		});
	}