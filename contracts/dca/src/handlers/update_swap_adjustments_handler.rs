use crate::{
    error::ContractError, state::swap_adjustments::update_swap_adjustments,
    types::position_type::PositionType,
};
use cosmwasm_std::{Decimal, DepsMut, Env, Response};

pub fn update_swap_adjustments_handler(
    deps: DepsMut,
    env: Env,
    position_type: PositionType,
    adjustments: Vec<(u8, Decimal)>,
) -> Result<Response, ContractError> {
    update_swap_adjustments(deps.storage, position_type, adjustments, env.block.time)?;
    Ok(Response::new())
}

#[cfg(test)]
mod update_swap_adjustments_tests {
    use super::*;
    use crate::{state::swap_adjustments::get_swap_adjustment, types::position_type::PositionType};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Decimal,
    };
    use std::str::FromStr;

    #[test]
    fn updates_swap_adjustments() {
        let old_adjustments = vec![
            (30, Decimal::from_str("0.921").unwrap()),
            (35, Decimal::from_str("0.926").unwrap()),
            (40, Decimal::from_str("0.931").unwrap()),
            (45, Decimal::from_str("0.936").unwrap()),
            (50, Decimal::from_str("0.941").unwrap()),
            (55, Decimal::from_str("0.946").unwrap()),
            (60, Decimal::from_str("0.951").unwrap()),
            (70, Decimal::from_str("0.961").unwrap()),
            (80, Decimal::from_str("0.971").unwrap()),
            (90, Decimal::from_str("0.981").unwrap()),
        ];

        let mut deps = mock_dependencies();

        update_swap_adjustments_handler(
            deps.as_mut(),
            mock_env(),
            PositionType::Enter,
            old_adjustments.clone(),
        )
        .unwrap();

        let new_adjustments = vec![
            (30, Decimal::from_str("1.921").unwrap()),
            (35, Decimal::from_str("1.926").unwrap()),
            (40, Decimal::from_str("1.931").unwrap()),
            (45, Decimal::from_str("1.936").unwrap()),
            (50, Decimal::from_str("1.941").unwrap()),
            (55, Decimal::from_str("1.946").unwrap()),
            (60, Decimal::from_str("1.951").unwrap()),
            (70, Decimal::from_str("1.961").unwrap()),
            (80, Decimal::from_str("1.971").unwrap()),
            (90, Decimal::from_str("1.981").unwrap()),
        ];

        update_swap_adjustments_handler(
            deps.as_mut(),
            mock_env(),
            PositionType::Enter,
            new_adjustments.clone(),
        )
        .unwrap();

        new_adjustments.iter().zip(old_adjustments.iter()).for_each(
            |((model, new_adjustment), (_, old_adjustment))| {
                let stored_adjustment = get_swap_adjustment(
                    deps.as_ref().storage,
                    PositionType::Enter,
                    *model,
                    mock_env().block.time,
                )
                .unwrap();
                assert_eq!(stored_adjustment, *new_adjustment);
                assert_ne!(stored_adjustment, *old_adjustment);
            },
        )
    }
}
