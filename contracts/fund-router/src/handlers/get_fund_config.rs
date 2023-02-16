use cosmwasm_std::Deps;
use cosmwasm_std::StdResult;

use crate::state::funds::get_current_fund;

use fund_core::msg::ConfigResponse as FundConfigResponse;
use fund_core::msg::QueryMsg as FundQueryMsg;

pub fn get_fund_config(deps: Deps) -> StdResult<FundConfigResponse> {
    let fund =
        get_current_fund(deps.storage)?.expect("a fund should exist to get its config");

    deps.querier
        .query_wasm_smart(fund.clone(), &FundQueryMsg::GetConfig {})
}
