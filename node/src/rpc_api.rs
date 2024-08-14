/// Defines the custom RPC interface for the lending pools
/// Specifies the structure and methods for the RPC interface

use jsonrpsee::proc_macros::rpc; // for creating the RPC macro
use serde::{Deserialize, Serialize}; // to enable serialization and deserialization of data

/// Defines the structure of the data returned by the RPC call
#[derive(Serialize, Deserialize, Debug)]
pub struct LendingPoolInfo {
    pub id: u32,
    pub asset: String,
    pub collateral_q: f64,
    pub utilization: f64,
    pub borrow_apy: f64,
    pub supply_apy: f64,
    pub collateral: bool,
    pub balance: u128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregatedTotals {
    pub total_supply: u128,
    pub total_borrow: u128,
}

#[rpc(server)] // Defines the trait as an RPC server
pub trait LendingPoolApi {
    #[method(name = "getLendingPools")]
    async fn get_lending_pools(&self) -> Result<(Vec<LendingPoolInfo>, AggregatedTotals), jsonrpsee::core::Error>;

    // takes a pool_id as input and returns the information for a single pool
    #[method(name = "getLendingPool")]
    async fn get_lending_pool(&self, pool_id: u32) -> Result<LendingPoolInfo, jsonrpsee::core::Error>;
} 
