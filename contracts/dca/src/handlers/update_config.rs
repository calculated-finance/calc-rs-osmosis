use crate::{
    constants::ONE_HUNDRED,
    error::ContractError,
    state::{Config, CONFIG},
    validation_helpers::assert_sender_is_admin,
};
use cosmwasm_std::{DepsMut, MessageInfo, Response, StdResult, Uint128};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    fee_collector: Option<String>,
    fee_percent: Option<Uint128>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.as_ref(), info.sender)?;

    if fee_percent.is_some() && fee_percent.unwrap() > ONE_HUNDRED {
        return Err(ContractError::CustomError {
            val: "fee_percent must be less than 100".to_string(),
        });
    }

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        if let Some(fee_collector) = fee_collector {
            config.fee_collector = deps.api.addr_validate(&fee_collector)?;
        }
        if let Some(fee_percent) = fee_percent {
            config.fee_percent = fee_percent
        }
        Ok(config)
    })?;

    Ok(Response::default()
        .add_attribute("method", "update_config")
        .add_attribute("fee_percent", config.fee_percent.to_string())
        .add_attribute("fee_collector", config.fee_collector.to_string()))
}
