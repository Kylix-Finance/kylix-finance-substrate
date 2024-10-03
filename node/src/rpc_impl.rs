use crate::rpc_api::LendingPoolApiServer;
use jsonrpsee::core::RpcResult;
use kylix_runtime::{
	lending::{
		AggregatedTotals, BorrowedAsset, CollateralAsset, LendingPoolInfo, SuppliedAsset, TotalBorrow, TotalCollateral, TotalDeposit,
	},
	AccountId, AssetId, LendingPoolApi, UserLTVInfo,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{traits::Block as BlockT, FixedU128};
use std::sync::Arc;

/// RPC method implementation for the Lending Pool API.
///
/// Provides methods to interact with lending pool functionality as defined in the runtime API.
/// Uses the `client` to access blockchain state and invoke the necessary runtime functions.
pub struct LendingPoolApiImpl<C, P> {
	/// Shared reference to the client for accessing blockchain state and runtime APIs.
	client: Arc<C>,
	/// Marker for associating the struct with a type `P`, used when the implementation depends on a phantom type parameter.
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> LendingPoolApiImpl<C, P> {
	/// Creates a new `LendingPoolApiImpl`.
	///
	/// # Arguments
	///
	/// * `client` - An `Arc` reference to the client providing access to runtime APIs and blockchain state.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, Block> LendingPoolApiServer for LendingPoolApiImpl<C, Block>
where
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	Block: BlockT,
	C::Api: LendingPoolApi<Block>,
{
	/// Retrieves lending pools and their aggregated totals.
	///
	/// Calls the `get_lending_pools` runtime API to fetch current lending pool states and their totals.
	///
	/// # Returns
	///
	/// `RpcResult<(Vec<LendingPoolInfo>, AggregatedTotals)>` containing:
	/// - A vector of lending pool details (`LendingPoolInfo`).
	/// - Aggregated totals across all lending pools (`AggregatedTotals`).
	///
	/// # Errors
	///
	/// Returns an error if the runtime API call fails.
	fn get_lending_pools(&self, asset_id: Option<AssetId>, account_id: Option<AccountId>) -> RpcResult<(Vec<LendingPoolInfo>, AggregatedTotals)> {
		// Access runtime API.
		let api = self.client.runtime_api();
		// Retrieve hash of the best block.
		let best_block_hash = self.client.info().best_hash;
		// Invoke the runtime method and handle errors.
		let result = api
			.get_lending_pools(best_block_hash, asset_id, account_id)
			.map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
		Ok(result)
	}

	/// Retrieves Loan-to-Value (LTV) information for a specific user.
	///
	/// # Returns
	///
	/// `RpcResult<UserLTVInfo>` containing the user's LTV information.
	fn get_user_ltv(&self, account: AccountId) -> RpcResult<UserLTVInfo> {
		let api = self.client.runtime_api();
		let best_block_hash = self.client.info().best_hash;

		let result = api
			.get_user_ltv(best_block_hash, account)
			.map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
		Ok(result)
	}

	/// Retrieves supplied assets and total deposits for a user.
	///
	/// # Returns
	///
	/// `RpcResult<(Vec<SuppliedAsset>, TotalDeposit)>` containing:
	/// - A vector of `SuppliedAsset`.
	/// - `TotalDeposit` of the user.
	fn get_asset_wise_supplies(
		&self,
		account: AccountId,
	) -> RpcResult<(Vec<SuppliedAsset>, TotalDeposit)> {
		let api = self.client.runtime_api();
		let best_block_hash = self.client.info().best_hash;

		let result = api
			.get_asset_wise_supplies(best_block_hash, account)
			.map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
		Ok(result)
	}

	/// Retrieves borrowed assets, collateral assets, total borrow, and total collateral for a user.
	///
	/// # Returns
	///
	/// `RpcResult<(Vec<BorrowedAsset>, Vec<CollateralAsset>, TotalBorrow, TotalCollateral)>` containing:
	/// - Vectors of `BorrowedAsset` and `CollateralAsset`.
	/// - `TotalBorrow` and `TotalCollateral` for the user.
	fn get_asset_wise_borrows_collaterals(
		&self,
		account: AccountId,
	) -> RpcResult<(Vec<BorrowedAsset>, Vec<CollateralAsset>, TotalBorrow, TotalCollateral)> {
		let api = self.client.runtime_api();
		let best_block_hash = self.client.info().best_hash;

		let result = api
			.get_asset_wise_borrows_collaterals(best_block_hash, account)
			.map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
		Ok(result)
	}

	/// Retrieves the price of a specific asset relative to an optional base asset.
	///
	/// # Returns
	///
	/// `RpcResult<Option<FixedU128>>` containing the asset price as `FixedU128`, if available.
	fn get_asset_price(
		&self,
		asset: AssetId,
		base_asset: Option<AssetId>,
	) -> RpcResult<Option<FixedU128>> {
		let api = self.client.runtime_api();
		let best_block_hash = self.client.info().best_hash;

		let result = api
			.get_asset_price(best_block_hash, asset, base_asset)
			.map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
		Ok(result)
	}
}
