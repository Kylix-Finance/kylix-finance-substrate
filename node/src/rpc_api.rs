/// Defines the custom RPC interface for the lending pools
/// Specifies the structure and methods for the RPC interface

use jsonrpsee::{core::RpcResult, proc_macros::rpc}; use kylix_runtime::{AggregatedTotals, LendingPoolInfo};

#[rpc(client, server)]
pub trait LendingPoolApi {
    #[method(name = "getLendingPools")]
    fn get_lending_pools(&self) -> RpcResult<(Vec<LendingPoolInfo>, AggregatedTotals)>;
}