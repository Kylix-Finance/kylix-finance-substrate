use crate::*;
use num_traits::{
	bounds::{LowerBounded, UpperBounded},
	One,
};
use sp_runtime::FixedU128;
use substrate_fixed::{
	transcendental::{cos, log2, pow},
	types::I64F64,
};

fn to_i64f64(x: FixedU128) -> I64F64 {
	I64F64::from_num(x.into_inner())
}

fn to_fixedu128(x: I64F64) -> FixedU128 {
	FixedU128::from_inner(x.to_num::<u128>())
}

fn log2_approx(x: FixedU128) -> Result<FixedU128, &'static str> {
	if x.is_zero() {
		return Err("Log of zero is undefined");
	}
	if x == FixedU128::one() {
		return Ok(FixedU128::zero());
	}
	log2(to_i64f64(x)).map(to_fixedu128).map_err(|_| "Log2 error")
}

fn pow_approx(base: FixedU128, exp: FixedU128) -> Result<FixedU128, &'static str> {
	if base.is_zero() && exp.is_zero() {
		return Ok(FixedU128::one());
	}
	if base.is_zero() {
		return Ok(FixedU128::zero());
	}
	if exp.is_zero() {
		return Ok(FixedU128::one());
	}
	if exp > FixedU128::from(1000u128) {
		return if base > FixedU128::one() {
			Ok(FixedU128::max_value())
		} else if base < FixedU128::one() {
			Ok(FixedU128::min_value())
		} else {
			Ok(FixedU128::one())
		};
	}
	pow(to_i64f64(base), to_i64f64(exp)).map(to_fixedu128).map_err(|_| "Pow error")
}

fn cos_approx(x: FixedU128) -> FixedU128 {
	to_fixedu128(cos(to_i64f64(x)))
}

#[derive(
	Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialOrd,
)]
pub struct InterestRateModel {
	pub y0: Rate, // Interest rate at 0% utilization
	pub y1: Rate, // Interest rate at 100% utilization
	pub xm: Rate, // Utilization at minimum interest
	pub ym: Rate, // Minimum interest rate
}

impl InterestRateModel {
	pub fn new(y0: Rate, y1: Rate, xm: Rate, ym: Rate) -> Self {
		Self { y0, y1, xm, ym }
	}

	pub fn calculate_cosine_interest(&self, utilization: Rate) -> Result<Rate, &'static str> {
		if utilization > Rate::one() {
			return Err("Utilization ratio must be between 0 and 1");
		}
		if utilization.is_zero() {
			return Ok(self.y0);
		}
		if utilization == Rate::one() {
			return Ok(self.y1);
		}

		let two = Rate::from_rational(2, 1);
		let pi = Rate::from_inner(3_141592653589793238u128); // π

		// Calculate n = -1 / log2(xm)
		let log2_xm = log2_approx(self.xm)?;
		let n = FixedU128::one()
			.checked_div(&log2_xm)
			.ok_or("Division by zero")?
			.checked_mul(&FixedU128::from_inner(u128::MAX))
			.ok_or("Multiplication overflow")?;

		// Calculate X = 2π * x^(-1/log2(xm))
		let x_pow_n = pow_approx(utilization, n)?;
		let x = two
			.checked_mul(&pi)
			.and_then(|v| v.checked_mul(&x_pow_n))
			.ok_or("Multiplication overflow")?;

		// Calculate cos(X)
		let cos_x = cos_approx(x);

		// Heaviside function
		let heaviside = if utilization > self.xm { Rate::one() } else { Rate::zero() };

		// Calculate the interest rate
		let one_plus_cos = Rate::one().checked_add(&cos_x).ok_or("Addition overflow")?;
		let one_minus_cos = Rate::one().checked_sub(&cos_x).ok_or("Subtraction overflow")?;

		let term1 = self
			.y0
			.checked_mul(&one_plus_cos)
			.and_then(|v| v.checked_mul(&(Rate::one() - heaviside)))
			.ok_or("Multiplication overflow")?;

		let term2 = self
			.y1
			.checked_mul(&one_plus_cos)
			.and_then(|v| v.checked_mul(&heaviside))
			.ok_or("Multiplication overflow")?;

		let term3 = self.ym.checked_mul(&one_minus_cos).ok_or("Multiplication overflow")?;

		let result = term1
			.checked_add(&term2)
			.and_then(|v| v.checked_add(&term3))
			.ok_or("Addition overflow")?;

		Ok(result.checked_div(&two).ok_or("Division by zero")?)
	}
}

impl Default for InterestRateModel {
	fn default() -> Self {
		Self {
			y0: Rate::from_rational(5, 100),  // 5% interest at 0% utilization
			y1: Rate::from_rational(15, 100), // 15% interest at 100% utilization
			xm: Rate::from_rational(80, 100), // Minimum at 80% utilization
			ym: Rate::from_rational(3, 100),  // 3% minimum interest rate
		}
	}
}
