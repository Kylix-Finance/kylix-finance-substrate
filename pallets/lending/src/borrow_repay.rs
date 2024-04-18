use crate::*;
/// Definition of the Borrow struct and its properties for an account
/// Balances are scaled-down balance taking borrow index into account
#[derive(
	Clone,
	Encode,
	Decode,
	Eq,
	PartialEq,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
	PartialOrd,
	DefaultNoBound,
)]
#[scale_info(skip_type_params(T))]
pub struct UserBorrow<T: Config> {
	pub borrowed_asset: AssetIdOf<T>,
	pub borrowed_balance: AssetBalanceOf<T>,
	pub collateral_asset: AssetIdOf<T>,
	pub collateral_balance: AssetBalanceOf<T>,
}

impl<T: Config> UserBorrow<T> {
	pub fn increase_borrow(&mut self, b: &Self) -> Result<(), Error<T>> {
		self.borrowed_balance = self
			.borrowed_balance
			.checked_add(&b.borrowed_balance)
			.ok_or(Error::<T>::OverflowError)?;
		self.collateral_balance = self
			.collateral_balance
			.checked_add(&b.collateral_balance)
			.ok_or(Error::<T>::OverflowError)?;

		Ok(())
	}
}
