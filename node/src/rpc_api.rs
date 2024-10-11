use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use kylix_runtime::{
	lending::{
		AggregatedTotals, BorrowedAsset, CollateralAsset, LendingPoolInfo, SuppliedAsset, TotalBorrow, TotalCollateral, TotalDeposit,
	}, AccountId, AssetId, Balance, UserLTVInfo
};
use sp_runtime::FixedU128;

/// Custom RPC interface for lending pool interactions.
///
/// This trait outlines the RPC methods available for interacting with lending pools 
/// on the blockchain. It is marked with the `#[rpc(client, server)]` attribute to 
/// automatically generate client and server implementations for each RPC method.
#[rpc(client, server)]
pub trait LendingPoolApi {
	/// Retrieves lending pool information and aggregated totals.
	///
	/// Returns details of all lending pools, including pool identifiers, associated assets, 
	/// and their aggregated totals.
	///
	/// # Returns
	///
	/// * `RpcResult<(Vec<LendingPoolInfo>, AggregatedTotals)>` - A result containing:
	///   - A vector of `LendingPoolInfo`, each representing a lending pool.
	///   - `AggregatedTotals`, which aggregates data for all lending pools.
	///
	/// # Errors
	///
	/// Returns an `RpcResult` containing an error if data retrieval fails, e.g., due to 
	/// blockchain state access issues.
	#[method(name = "getLendingPools")]
	fn get_lending_pools(
		&self,
		asset_id: Option<AssetId>,
		account_id: Option<AccountId>,
	) -> RpcResult<(Vec<LendingPoolInfo>, AggregatedTotals)>;

	/// Retrieves the Loan-to-Value (LTV) information for a user.
	///
	/// # Parameters
	///
	/// * `account` - The `AccountId` of the user.
	///
	/// # Returns
	///
	/// * `RpcResult<UserLTVInfo>` - The LTV information for the user.
	#[method(name = "getUserLtv")]
	fn get_user_ltv(&self, account: AccountId) -> RpcResult<UserLTVInfo>;

	/// Retrieves the supplied assets and total deposits for a user.
	///
	/// # Parameters
	///
	/// * `account` - The `AccountId` of the user.
	///
	/// # Returns
	///
	/// * `RpcResult<(Vec<SuppliedAsset>, TotalDeposit)>` - A tuple containing:
	///   - A vector of `SuppliedAsset`.
	///   - The `TotalDeposit` made by the user.
	#[method(name = "getAssetWiseSupplies")]
	fn get_asset_wise_supplies(
		&self,
		account: AccountId,
	) -> RpcResult<(Vec<SuppliedAsset>, TotalDeposit)>;

	/// Retrieves borrowed assets, collaterals, total borrow, and total collateral for a user.
	///
	/// # Parameters
	///
	/// * `account` - The `AccountId` of the user.
	///
	/// # Returns
	///
	/// * `RpcResult<(Vec<BorrowedAsset>, Vec<CollateralAsset>, TotalBorrow, TotalCollateral)>` - A tuple containing:
	///   - A vector of `BorrowedAsset`.
	///   - A vector of `CollateralAsset`.
	///   - `TotalBorrow` and `TotalCollateral` values for the user.
	#[method(name = "getAssetWiseBorrowsCollaterals")]
	fn get_asset_wise_borrows_collaterals(
		&self,
		account: AccountId,
	) -> RpcResult<(Vec<BorrowedAsset>, Vec<CollateralAsset>, TotalBorrow, TotalCollateral)>;

	/// Fetches the price of a specified asset relative to a base asset.
	///
	/// # Parameters
	///
	/// * `asset` - The `AssetId` for which the price is to be retrieved.
	/// * `base_asset` - An optional `AssetId` representing the base asset for the price comparison.
	///
	/// # Returns
	///
	/// * `RpcResult<Option<FixedU128>>` - The price as a `FixedU128`, if available.
	#[method(name = "getAssetPrice")]
	fn get_asset_price(
		&self,
		asset: AssetId,
		base_asset: Option<AssetId>,
	) -> RpcResult<Option<FixedU128>>;

	/// Estimates the amount of collateral required for a specified borrow amount and asset.
	///
	/// This method calculates the required collateral that needs to be supplied for borrowing
	/// a specific amount of an asset.
	///
	/// # Parameters
	///
	/// * `borrow_asset` - The `AssetId` of the asset the user wants to borrow.
	/// * `borrow_amount` - The amount of the asset to be borrowed.
	/// * `collateral_asset` - The `AssetId` of the asset to be used as collateral.
	///
	/// # Returns
	///
	/// * `RpcResult<Option<Balance>>` - The estimated amount of collateral required as `Balance`, 
	///   if the estimation is available.
	///
	/// # Errors
	///
	/// Returns an error if the estimation process fails or cannot retrieve the necessary data 
	/// from the runtime.
	#[method(name = "getEstimateCollateralAmount")]
	fn get_estimate_collateral_amount(
		&self,
		borrow_asset: AssetId,
		borrow_amount: Balance,
		collateral_asset: AssetId,
	) -> RpcResult<Option<Balance>>;
}
