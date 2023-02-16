use base::ContractError;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, MessageInfo, Response, SubMsg,
    WasmMsg::Instantiate as WasmInstantiate,
};

use crate::{
    contract::AFTER_INSTANTIATE_FUND_FOR_MIGRATION_REPLY_ID,
    state::{
        cache::{Cache, CACHE},
        config::get_config,
    },
    validation_helpers::assert_sender_is_router_owner_or_admin,
};

use fund_core::msg::{ConfigResponse as FundConfigResponse, InstantiateMsg as FundInstantiateMsg};

use fund_router::msg::QueryMsg as RouterQueryMsg;

pub fn migrate_fund(
    deps: DepsMut,
    info: MessageInfo,
    router: Addr,
) -> Result<Response, ContractError> {
    assert_sender_is_router_owner_or_admin(
        deps.storage,
        info.sender,
        &deps
            .querier
            .query_wasm_smart(router.clone(), &RouterQueryMsg::GetConfig {})?,
    )?;

    CACHE.save(
        deps.storage,
        &Cache {
            owner: None,
            router_address: Some(router.clone()),
        },
    )?;

    let config = get_config(deps.storage)?;

    let get_fund_config_response: FundConfigResponse = deps
        .querier
        .query_wasm_smart(router.clone(), &RouterQueryMsg::GetFundConfig {})?;

    let fund_instantiate_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmInstantiate {
            admin: None,
            label: format!("CALC-MF-FUND"),
            code_id: config.fund_code_id,
            funds: vec![],
            msg: to_binary(&FundInstantiateMsg {
                router: router.clone(),
                swapper: get_fund_config_response.config.swapper,
                base_denom: get_fund_config_response.config.base_denom,
            })?,
        }),
        AFTER_INSTANTIATE_FUND_FOR_MIGRATION_REPLY_ID,
    );

    Ok(Response::new()
        .add_attribute("method", "migrate_fund")
        .add_submessage(fund_instantiate_msg))
}
