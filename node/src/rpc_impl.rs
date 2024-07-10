/// Implementation for the LendingPoolApi defined in api.rs

use async_trait::async_trait;
use crate::rpc_api::{LendingPoolApiServer, LendingPoolInfo, AggregatedTotals};

#[derive(Clone)]
pub struct LendingPoolRpcImpl; // empty struct that will implement the RPC trait

/// Implements the get_lending_pools method defined in the LendingPoolApi trait
#[async_trait]
impl LendingPoolApiServer for LendingPoolRpcImpl {
    async fn get_lending_pools(&self) -> Result<(Vec<LendingPoolInfo>, AggregatedTotals), jsonrpsee::core::Error> {
        // Mock data
        let pools = vec![
            LendingPoolInfo { 
                id: 1, 
                asset: "TokenA".to_string(), 
                collateral_q: 50.0, 
                utilization: 60.0, 
                borrow_apy: 5.0, 
                supply_apy: 2.0, 
                collateral: true, 
                balance: 1000 
            },
            LendingPoolInfo { 
                id: 2, 
                asset: "TokenB".to_string(), 
                collateral_q: 70.0, 
                utilization: 80.0, 
                borrow_apy: 6.0, 
                supply_apy: 3.0, 
                collateral: true, 
                balance: 2000 
            },
            LendingPoolInfo { 
                id: 3, 
                asset: "TokenC".to_string(), 
                collateral_q: 30.0, 
                utilization: 40.0, 
                borrow_apy: 4.0, 
                supply_apy: 1.5, 
                collateral: false, 
                balance: 3000 
            },
            LendingPoolInfo { 
                id: 4, 
                asset: "TokenD".to_string(), 
                collateral_q: 90.0, 
                utilization: 70.0, 
                borrow_apy: 7.0, 
                supply_apy: 4.0, 
                collateral: true, 
                balance: 4000 
            },
        ];
        
        // Calculate aggregated totals
        let total_supply: u128 = pools.iter().map(|p| p.balance).sum();
        let total_borrow: u128 = pools.iter().map(|p| p.balance).sum(); // Adjust this calculation if needed
        
        let aggregated_totals = AggregatedTotals {
            total_supply,
            total_borrow,
        };
        
        Ok((pools, aggregated_totals))
    }
}

impl LendingPoolRpcImpl {
    pub fn new() -> Self {
        Self {}
    }
}
