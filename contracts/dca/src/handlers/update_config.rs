use crate::{
    error::ContractError,
    state::{Config, CONFIG},
    validation_helpers::assert_sender_is_admin,
};
use cosmwasm_std::{Decimal, DepsMut, MessageInfo, Response, StdResult};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    fee_collector: Option<String>,
    fee_rate: Option<Decimal>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.as_ref(), info.sender)?;

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        if let Some(fee_collector) = fee_collector {
            config.fee_collector = deps.api.addr_validate(&fee_collector)?;
        }
        if let Some(fee_rate) = fee_rate {
            config.fee_rate = fee_rate
        }
        Ok(config)
    })?;

    Ok(Response::default()
        .add_attribute("method", "update_config")
        .add_attribute("fee_rate", config.fee_rate.to_string())
        .add_attribute("fee_collector", config.fee_collector.to_string()))
}
