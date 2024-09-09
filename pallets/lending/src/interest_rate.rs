use crate::*;
use core::f64::consts::PI;
use num_traits::One;
use substrate_fixed::{
	transcendental::{cos, log2, pow},
	types::I64F64,
};

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

		let y0 = I64F64::from_num(self.y0.into_inner());
		let y1 = I64F64::from_num(self.y1.into_inner());
		let xm = I64F64::from_num(self.xm.into_inner());
		let ym = I64F64::from_num(self.ym.into_inner());
		let utilization = I64F64::from_num(utilization.into_inner());

		// Calculate n = -1 / log2(xm)
		let log2_xm: I64F64 = log2(xm).map_err(|_| "Logarithm calculation failed")?;
		let n = I64F64::from_num(-1) / log2_xm;

		// Calculate X = 2π * x^(-1/log2(xm))
		let pi = I64F64::from_num(PI);
		let two_pi = pi * I64F64::from_num(2);
		let x_pow_n: I64F64 = pow(utilization, n).map_err(|_| "Power error")?;
		let x = two_pi * x_pow_n;

		// Calculate cos(X)
		let cos_x = cos(x);

		// Heaviside function
		let heaviside = if utilization > xm { I64F64::from_num(1) } else { I64F64::from_num(0) };

		// Calculate the interest rate
		let one = I64F64::from_num(1);
		let one_plus_cos = one + cos_x;
		let one_minus_cos = one - cos_x;

		let term1 = y0 * one_plus_cos * (one - heaviside);
		let term2 = y1 * one_plus_cos * heaviside;
		let term3 = ym * one_minus_cos;

		let result: I64F64 = (term1 + term2 + term3) / I64F64::from_num(2);

		// Convert back to Rate (FixedU128)
		Ok(Rate::from_inner(result.to_num::<u128>()))
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
