use std::sync::Arc;
use jsonrpsee::core::RpcResult;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use kylix_runtime::{LendingPoolApi,AggregatedTotals, LendingPoolInfo};
use crate::rpc_api::LendingPoolApiServer;

pub struct LendingPoolApiImpl<C, P> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> LendingPoolApiImpl<C, P> {
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
    fn get_lending_pools(&self) -> RpcResult<(Vec<LendingPoolInfo>, AggregatedTotals)> {
        let api = self.client.runtime_api();
        let best_block_hash = self.client.info().best_hash;
        let result = api.get_lending_pools(best_block_hash)
            .map_err(|e| jsonrpsee::core::Error::Custom(e.to_string()))?;
        Ok(result)
    }
}
