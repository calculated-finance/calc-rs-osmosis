use crate::{
    error::ContractError,
    state::{Config, CONFIG},
    validation_helpers::assert_sender_is_admin,
};
use cosmwasm_std::{Addr, Decimal, DepsMut, MessageInfo, Response, StdResult};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    fee_collector: Option<Addr>,
    fee_percent: Option<Decimal>,
    staking_router_address: Option<Addr>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    if fee_percent.is_some() && fee_percent.unwrap() > Decimal::percent(100) {
        return Err(ContractError::CustomError {
            val: "fee_percent must be less than 100".to_string(),
        });
    }

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        if let Some(fee_collector) = fee_collector {
            deps.api.addr_validate(&fee_collector.to_string())?;
            config.fee_collector = fee_collector;
        }
        if let Some(fee_percent) = fee_percent {
            config.fee_percent = fee_percent
        }
        if let Some(staking_router_address) = staking_router_address {
            deps.api
                .addr_validate(&staking_router_address.to_string())?;
            config.staking_router_address = staking_router_address;
        }

        Ok(config)
    })?;

    Ok(Response::default()
        .add_attribute("method", "update_config")
        .add_attribute("fee_percent", config.fee_percent.to_string())
        .add_attribute("fee_collector", config.fee_collector.to_string())
        .add_attribute(
            "staking_router_address",
            config.staking_router_address.to_string(),
        ))
}
