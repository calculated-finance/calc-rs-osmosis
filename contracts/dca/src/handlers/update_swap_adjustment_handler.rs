use crate::{
    error::ContractError,
    helpers::validation::{assert_sender_is_executor, assert_swap_adjustment_value_is_valid},
    state::swap_adjustments::update_swap_adjustment,
    types::swap_adjustment_strategy::SwapAdjustmentStrategy,
};
use cosmwasm_std::{Decimal, DepsMut, Env, MessageInfo, Response};

pub fn update_swap_adjustment_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    strategy: SwapAdjustmentStrategy,
    value: Decimal,
) -> Result<Response, ContractError> {
    assert_sender_is_executor(deps.storage, &env, &info.sender)?;
    assert_swap_adjustment_value_is_valid(&strategy, value)?;
    update_swap_adjustment(deps.storage, strategy.clone(), value, env.block.time)?;

    Ok(Response::new()
        .add_attribute("strategy", format!("{:?}", strategy))
        .add_attribute("value", value.to_string()))
}

#[cfg(test)]
mod update_swap_adjustments_tests {
    use super::*;
    use crate::{
        state::swap_adjustments::get_swap_adjustment,
        tests::{helpers::instantiate_contract, mocks::ADMIN},
        types::{
            position_type::PositionType,
            swap_adjustment_strategy::{BaseDenom, SwapAdjustmentStrategy},
        },
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Decimal,
    };

    #[test]
    fn with_non_executor_sender_fails() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info);

        let strategy = SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        };

        let value = Decimal::percent(125);

        let err = update_swap_adjustment_handler(
            deps.as_mut(),
            env,
            mock_info("not-an-executor", &[]),
            strategy,
            value,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Unauthorized");
    }

    #[test]
    fn with_invalid_value_fails() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let strategy = SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        };

        let value = Decimal::percent(10);

        let err = update_swap_adjustment_handler(
            deps.as_mut(),
            env,
            info,
            strategy.clone(),
            value,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Error: swap adjustment value for strategy {:?} must be between {} and {}",
                strategy,
                strategy.min_adjustment(),
                strategy.max_adjustment()
            )
        );
    }

    #[test]
    fn updates_swap_adjustment() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let strategy = SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        };

        let old_value = Decimal::percent(125);

        update_swap_adjustment_handler(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            strategy.clone(),
            old_value,
        )
        .unwrap();

        let new_value = Decimal::percent(95);

        update_swap_adjustment_handler(
            deps.as_mut(),
            env.clone(),
            info,
            strategy.clone(),
            new_value,
        )
        .unwrap();

        let stored_adjustment =
            get_swap_adjustment(deps.as_ref().storage, strategy, env.block.time);

        assert_ne!(stored_adjustment, old_value);
        assert_eq!(stored_adjustment, new_value);
    }
}
