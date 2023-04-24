use crate::{
    error::ContractError, state::swap_adjustments::update_swap_adjustment,
    types::swap_adjustment_strategy::SwapAdjustmentStrategy,
};
use cosmwasm_std::{Decimal, DepsMut, Env, Response};

pub fn update_swap_adjustment_handler(
    deps: DepsMut,
    env: Env,
    strategy: SwapAdjustmentStrategy,
    value: Decimal,
) -> Result<Response, ContractError> {
    update_swap_adjustment(deps.storage, strategy, value, env.block.time)?;
    Ok(Response::new())
}

#[cfg(test)]
mod update_swap_adjustments_tests {
    use super::*;
    use crate::{
        state::swap_adjustments::get_swap_adjustment,
        types::{
            position_type::PositionType,
            swap_adjustment_strategy::{BaseDenom, SwapAdjustmentStrategy},
        },
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Decimal,
    };

    #[test]
    fn updates_swap_adjustments() {
        let mut deps = mock_dependencies();

        let strategy = SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        };

        let old_value = Decimal::percent(125);
        update_swap_adjustment_handler(deps.as_mut(), mock_env(), strategy.clone(), old_value)
            .unwrap();

        let new_value = Decimal::percent(95);
        update_swap_adjustment_handler(deps.as_mut(), mock_env(), strategy.clone(), new_value)
            .unwrap();

        let stored_adjustment =
            get_swap_adjustment(deps.as_ref().storage, strategy, mock_env().block.time).unwrap();

        assert_ne!(stored_adjustment, old_value);
        assert_eq!(stored_adjustment, new_value);
    }
}
