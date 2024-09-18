use crate::{interest_rate::InterestRateModel, tests::mock::*};
use num_traits::{One, Zero};
use sp_runtime::{assert_eq_error_rate, FixedU128};

fn create_default_model() -> InterestRateModel {
	InterestRateModel::default()
}

// Helper function to create a small FixedU128 value for error margin
fn small_error() -> FixedU128 {
	FixedU128::from_inner(1_000_000) // This is approximately 1e-12 in FixedU128
}

#[test]
fn test_zero_utilization() {
	let model = create_default_model();
	let result = model.calculate_cosine_interest(Rate::zero()).unwrap();
	assert_eq_error_rate!(result, model.y0, small_error());
}

#[test]
fn test_full_utilization() {
	let model = create_default_model();
	let result = model.calculate_cosine_interest(Rate::one()).unwrap();
	assert_eq_error_rate!(result, model.y1, small_error());
}

#[test]
fn test_minimum_interest_point() {
	let model = create_default_model();
	let result = model.calculate_cosine_interest(model.xm).unwrap();
	assert_eq_error_rate!(result, model.ym, small_error());
}

#[test]
fn test_mid_point_utilization() {
	let model = create_default_model();
	let mid_point = Rate::from_rational(50, 100);
	let result = model.calculate_cosine_interest(mid_point).unwrap();
	// The result should be between ym and y1
	assert!(result >= model.ym && result <= model.y1);
}

#[test]
fn test_invalid_utilization() {
	let model = create_default_model();
	// Test above 100% utilization
	assert!(model.calculate_cosine_interest(Rate::from_rational(101, 100)).is_err());

	// Test at exactly 100% utilization (should be OK)
	assert!(model.calculate_cosine_interest(Rate::one()).is_ok());

	// Test at exactly 0% utilization (should be OK)
	assert!(model.calculate_cosine_interest(Rate::zero()).is_ok());

	// Test very small positive number (should be OK)
	assert!(model.calculate_cosine_interest(Rate::from_inner(1)).is_ok());
}

#[test]
fn test_custom_model() {
	let custom_model = InterestRateModel::new(
		Rate::from_rational(1, 100),  // 1% at 0% utilization
		Rate::from_rational(20, 100), // 20% at 100% utilization
		Rate::from_rational(70, 100), // Minimum at 70% utilization
		Rate::from_rational(5, 1000), // 0.5% minimum interest rate
	);

	let result = custom_model.calculate_cosine_interest(Rate::from_rational(35, 100)).unwrap();
	// The result should be between ym and y1
	assert!(result >= custom_model.ym && result <= custom_model.y0);
}

#[test]
fn test_calculate_cosine_interest() {
	let model = InterestRateModel::default();
	let utilization = Rate::from_rational(3333, 10000); // 33.33%
	let interest_rate = model.calculate_cosine_interest(utilization).unwrap();
	assert!(interest_rate > Rate::zero());
}
