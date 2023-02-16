use base::ContractError;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, MessageInfo, Response, SubMsg,
    WasmMsg::Execute as WasmExecuteMsg,
};

use crate::{
    state::funds::{get_current_fund, save_fund},
    validation_helpers::assert_sender_is_factory,
};

use fund_core::msg::ExecuteMsg as FundExecuteMsg;

pub fn assign_fund(
    deps: DepsMut,
    info: MessageInfo,
    fund_address: Addr,
) -> Result<Response, ContractError> {
    assert_sender_is_factory(deps.storage, info.sender.clone())?;
    deps.api.addr_validate(fund_address.as_str())?;

    let existing_fund = get_current_fund(deps.storage)?;

    let mut response = Response::new()
        .add_attribute("method", "assign_fund")
        .add_attribute("fund_address", fund_address.to_string());

    if existing_fund.is_some() {
        response = response.add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmExecuteMsg {
            contract_addr: existing_fund.unwrap().to_string(),
            funds: vec![],
            msg: to_binary(&FundExecuteMsg::Migrate {
                new_fund_address: fund_address.clone(),
            })?,
        })));
    }

    save_fund(deps.storage, fund_address.clone())?;
    Ok(response)
}
