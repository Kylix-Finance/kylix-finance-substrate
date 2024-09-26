use crate::{tests::mock::*, AssetPool};
use frame_support::assert_ok;
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
