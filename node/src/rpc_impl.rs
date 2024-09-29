use std::sync::Arc;
use jsonrpsee::core::RpcResult;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use kylix_runtime::{AccountId, AggregatedTotals, BorrowedAsset, CollateralAsset, LendingPoolApi, LendingPoolInfo, SuppliedAsset, TotalBorrow, TotalCollateral, TotalDeposit, UserLTVInfo};
use crate::rpc_api::LendingPoolApiServer;

/// Implementation of the RPC methods for the Lending Pool API.
///
/// This struct provides the necessary methods to interact with the lending pool
/// functionality exposed by the runtime. It leverages the client to access the
/// runtime API.
pub struct LendingPoolApiImpl<C, P> {
    /// A shared reference to the client. The client provides access to the
    /// blockchain state and runtime APIs.
    client: Arc<C>,
    /// A marker to associate the struct with a specific type `P`. This is
    /// useful when the implementation depends on a phantom type parameter.
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> LendingPoolApiImpl<C, P> {
    /// Creates a new instance of `LendingPoolApiImpl`.
    ///
    /// # Arguments
    ///
    /// * `client` - An `Arc` reference to the client that provides access to
    ///   the runtime APIs and blockchain state.
    ///
    /// # Returns
    ///
    /// A new instance of `LendingPoolApiImpl`.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block> LendingPoolApiServer for LendingPoolApiImpl<C, Block>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    Block: BlockT,
    C::Api: LendingPoolApi<Block>,
{
    /// Retrieves the list of lending pools along with aggregated totals.
    ///
    /// This method calls the `get_lending_pools` runtime API to fetch the
    /// current state of lending pools and their aggregated totals.
    ///
    /// # Returns
    ///
    /// A `RpcResult` containing a tuple:
    /// - `Vec<LendingPoolInfo>`: A vector of lending pool information.
    /// - `AggregatedTotals`: Aggregated totals across all lending pools.
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime API call fails.
    fn get_lending_pools(&self) -> RpcResult<(Vec<LendingPoolInfo>, AggregatedTotals)> {
        // Access the runtime API.
        let api = self.client.runtime_api();
        
        // Retrieve the hash of the best (most recent) block.
        let best_block_hash = self.client.info().best_hash;
        
        // Call the `get_lending_pools` method from the runtime API.
        let result = api.get_lending_pools(best_block_hash)
            .map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
        
        // Return the result.
        Ok(result)
    }

    fn get_user_ltv(&self, account: AccountId) -> RpcResult<UserLTVInfo> {
        let api = self.client.runtime_api();
        let best_block_hash = self.client.info().best_hash;
        
        let result = api.get_user_ltv(best_block_hash, account)
            .map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
        
        Ok(result)
    }

    fn get_asset_wise_supplies(&self, account: AccountId) -> RpcResult<(Vec<SuppliedAsset>, TotalDeposit)> {
        let api = self.client.runtime_api();
        let best_block_hash = self.client.info().best_hash;
        
        let result = api.get_asset_wise_supplies(best_block_hash, account)
            .map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
        
        Ok(result)
    }

    fn get_asset_wise_borrows_collaterals(&self, account: AccountId) -> RpcResult<(Vec<BorrowedAsset>, Vec<CollateralAsset>, TotalBorrow, TotalCollateral)> {
        let api = self.client.runtime_api();
        let best_block_hash = self.client.info().best_hash;
        
        let result = api.get_asset_wise_borrows_collaterals(best_block_hash, account)
            .map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
        
        Ok(result)
    }
}
