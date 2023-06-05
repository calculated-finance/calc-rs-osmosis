use cosmwasm_std::{DepsMut, Response, Uint128};

use crate::{
    error::ContractError,
    state::{
        pairs::find_pair,
        vaults::{get_vault, update_vault},
    },
    types::swap_adjustment_strategy::SwapAdjustmentStrategy,
};

pub fn fix_position_type(deps: DepsMut, vault_id: Uint128) -> Result<Response, ContractError> {
    let mut vault = get_vault(deps.storage, vault_id)?;

    let mut response = Response::new()
        .add_attribute("fix_position_type", "true")
        .add_attribute("vault_id", vault_id.to_string());

    if let Some(SwapAdjustmentStrategy::RiskWeightedAverage {
        model_id,
        base_denom,
        position_type,
    }) = vault.swap_adjustment_strategy.clone()
    {
        response = response.add_attribute("old_position_type", format!("{:?}", position_type));

        let pair = find_pair(deps.storage, vault.denoms())?;
        let new_position_type = pair.position_type(vault.get_swap_denom());

        vault.swap_adjustment_strategy = Some(SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id,
            base_denom,
            position_type: new_position_type.clone(),
        });

        update_vault(deps.storage, vault)?;

        response = response.add_attribute("new_position_type", format!("{:?}", new_position_type));
    };

    Ok(response)
}
