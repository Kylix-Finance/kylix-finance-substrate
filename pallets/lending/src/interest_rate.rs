use crate::*;
use num_traits::{
	bounds::{LowerBounded, UpperBounded},
	One,
};

fn log2_approx(x: FixedU128) -> FixedU128 {
	if x <= FixedU128::zero() {
		return FixedU128::zero(); // or handle error
	}

	let mut result = FixedU128::zero();
	let mut x = x;
	let two = FixedU128::from_inner(2u128 << 64);

	while x >= two {
		result = result.checked_add(&FixedU128::one()).unwrap_or(FixedU128::max_value());
		x = x.checked_div(&two).unwrap_or(FixedU128::zero());
	}

	let one = FixedU128::one();
	let mut y = one;

	for _ in 0..64 {
		y = y.checked_div(&two).unwrap_or(FixedU128::zero());
		if x >= one.checked_add(&y).unwrap_or(FixedU128::max_value()) {
			x = x
				.checked_div(&one.checked_add(&y).unwrap_or(FixedU128::max_value()))
				.unwrap_or(FixedU128::zero());
			result = result.checked_add(&y).unwrap_or(FixedU128::max_value());
		}
	}

	result
}

// Exponential function approximation using Taylor series
fn exp_approx(x: FixedU128) -> FixedU128 {
	let mut result = FixedU128::one();
	let mut term = FixedU128::one();
	let mut n = FixedU128::one();

	for _ in 1..10 {
		// Adjust the number of iterations for desired precision
		term = term
			.checked_mul(&x)
			.unwrap_or(FixedU128::zero())
			.checked_div(&n)
			.unwrap_or(FixedU128::zero());
		result = result.checked_add(&term).unwrap_or(FixedU128::max_value());
		n = n.checked_add(&FixedU128::one()).unwrap_or(FixedU128::max_value());
	}

	result
}

// Natural logarithm approximation using Taylor series
fn ln_approx(x: FixedU128) -> FixedU128 {
	if x <= FixedU128::zero() {
		return FixedU128::min_value(); // or handle error
	}

	let y = (x - FixedU128::one()) / (x + FixedU128::one());
	let mut result = FixedU128::zero();
	let mut term = y;
	let two = FixedU128::from_inner(2u128 << 64);

	for n in 1..10 {
		// Adjust the number of iterations for desired precision
		result = result
			.checked_add(&(term / FixedU128::from_u32(2 * n - 1)))
			.unwrap_or(FixedU128::max_value());
		term = term
			.checked_mul(&y)
			.unwrap_or(FixedU128::zero())
			.checked_mul(&y)
			.unwrap_or(FixedU128::zero());
	}

	result.checked_mul(&two).unwrap_or(FixedU128::max_value())
}

fn pow_approx(base: FixedU128, exp: FixedU128) -> FixedU128 {
	if base == FixedU128::zero() {
		return if exp == FixedU128::zero() { FixedU128::one() } else { FixedU128::zero() };
	}

	exp_approx(ln_approx(base).checked_mul(&exp).unwrap_or(FixedU128::zero()))
}

fn cos_approx(mut x: FixedU128) -> FixedU128 {
	let pi = FixedU128::from_inner(3_141592653589793238u128);
	let two_pi = pi.checked_mul(&FixedU128::from_u32(2)).unwrap_or(FixedU128::max_value());

	// Normalize x to be between 0 and 2π
	while x > two_pi {
		x = x.checked_sub(&two_pi).unwrap_or(FixedU128::zero());
	}
	while x < FixedU128::zero() {
		x = x.checked_add(&two_pi).unwrap_or(FixedU128::max_value());
	}

	let mut result = FixedU128::one();
	let mut term = FixedU128::one();
	let mut n = FixedU128::zero();
	let neg_one = FixedU128::from_inner(u128::MAX); // -1 in FixedU128

	for _ in 1..10 {
		// Adjust the number of iterations for desired precision
		n = n.checked_add(&FixedU128::from_u32(2)).unwrap_or(FixedU128::max_value());
		term = term
			.checked_mul(&x)
			.unwrap_or(FixedU128::zero())
			.checked_mul(&x)
			.unwrap_or(FixedU128::zero())
			.checked_div(&(n * (n - FixedU128::one())))
			.unwrap_or(FixedU128::zero())
			.checked_mul(&neg_one)
			.unwrap_or(FixedU128::zero());
		result = result.checked_add(&term).unwrap_or(FixedU128::max_value());
	}

	result
}

#[derive(
	Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialOrd,
)]
pub struct InterestRateModel {
	y0: Rate, // Interest rate at 0% utilization
	y1: Rate, // Interest rate at 100% utilization
	xm: Rate, // Utilization at minimum interest
	ym: Rate, // Minimum interest rate
}

impl InterestRateModel {
	pub fn new(y0: Rate, y1: Rate, xm: Rate, ym: Rate) -> Self {
		Self { y0, y1, xm, ym }
	}

	pub fn calculate_cosine_interest(&self, utilization: Rate) -> Result<Rate, &'static str> {
		if utilization > Rate::one() || utilization < Rate::zero() {
			return Err("Utilization ratio must be between 0 and 1");
		}

		let two = Rate::from_rational(2, 1); // 2.0
		let pi = Rate::from_inner(3_141592653589793238u128); // π

		// Calculate n = -1 / log2(xm)
		let log2_xm = log2_approx(self.xm);
		let n = FixedU128::one()
			.checked_div(&log2_xm)
			.ok_or("Division by zero")?
			.checked_mul(&FixedU128::from_inner(u128::MAX)) // Negate by multiplying with -1
			.ok_or("Multiplication overflow")?;

		// Calculate x^n
		let x_pow_n = pow_approx(utilization, n);

		// Calculate cos(2πx^n)
		let cos_term = cos_approx(
			two.checked_mul(&pi)
				.ok_or("Multiplication overflow")?
				.checked_mul(&x_pow_n)
				.ok_or("Multiplication overflow")?,
		);

		// Heaviside function
		let heaviside = if utilization > self.xm { Rate::one() } else { Rate::zero() };

		// Calculate the interest rate
		let term1 = self
			.y0
			.checked_mul(&(Rate::one() + cos_term))
			.ok_or("Multiplication overflow")?
			.checked_mul(&(Rate::one() - heaviside))
			.ok_or("Multiplication overflow")?;

		let term2 = self
			.y1
			.checked_mul(&(Rate::one() + cos_term))
			.ok_or("Multiplication overflow")?
			.checked_mul(&heaviside)
			.ok_or("Multiplication overflow")?;

		let term3 = self
			.ym
			.checked_mul(&(Rate::one() - cos_term))
			.ok_or("Multiplication overflow")?;

		let result = term1
			.checked_add(&term2)
			.ok_or("Addition overflow")?
			.checked_add(&term3)
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
