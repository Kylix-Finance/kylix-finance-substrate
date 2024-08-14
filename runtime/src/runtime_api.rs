use sp_std::vec::Vec;
use sp_runtime::DispatchError;
use crate::LendingPoolInfo;
use crate::AggregatedTotals;

sp_api::decl_runtime_apis! {
    pub trait LendingPoolRuntimeApi {
        fn get_lending_pools() -> Result<(Vec<LendingPoolInfo>, AggregatedTotals), DispatchError>;
    }
}
