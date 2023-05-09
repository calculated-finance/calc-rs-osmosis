use crate::{
    helpers::{
        fees::get_performance_fee, price::query_belief_price, vault::get_performance_factor,
    },
    msg::VaultPerformanceResponse,
    state::{pairs::find_pair, vaults::get_vault},
};
use cosmwasm_std::{Deps, Env, StdError, StdResult, Uint128};

pub fn get_vault_performance_handler(
    deps: Deps,
    env: &Env,
    vault_id: Uint128,
) -> StdResult<VaultPerformanceResponse> {
    let vault = get_vault(deps.storage, vault_id)?;

    let pair = find_pair(deps.storage, vault.denoms())?;

    let current_price = query_belief_price(&deps, env, &pair, vault.get_swap_denom())?;

    vault.performance_assessment_strategy.clone().map_or(
        Err(StdError::GenericErr {
            msg: format!(
                "Vault {} does not have a performance assessment strategy",
                vault_id
            ),
        }),
        |_| {
            Ok(VaultPerformanceResponse {
                fee: get_performance_fee(&vault, current_price)?,
                factor: get_performance_factor(&vault, current_price)?,
            })
        },
    )
}

#[cfg(test)]
mod get_vault_performance_tests {
    use super::get_vault_performance_handler;
    use crate::{
        constants::{ONE, TEN},
        tests::{
            helpers::{instantiate_contract, setup_vault},
            mocks::{calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO},
        },
        types::{
            performance_assessment_strategy::PerformanceAssessmentStrategy,
            swap_adjustment_strategy::SwapAdjustmentStrategy, vault::Vault,
        },
    };
    use cosmwasm_std::{
        testing::{mock_env, mock_info},
        Coin, Decimal,
    };

    #[test]
    fn if_vault_has_no_performance_assessment_strategy_fails() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let err = get_vault_performance_handler(deps.as_ref(), &env, vault.id).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Generic error: Vault 0 does not have a performance assessment strategy"
        );
    }

    #[test]
    fn performance_fee_and_factor_match() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let standard_received_amount = TEN - ONE;

        let performance_assessment_strategy = PerformanceAssessmentStrategy::CompareToStandardDca {
            swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            received_amount: Coin::new(standard_received_amount.into(), DENOM_STAKE),
        };

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swapped_amount: Coin::new(TEN.into(), DENOM_STAKE),
                received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                escrowed_amount: Coin::new(TEN.into(), DENOM_STAKE),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(performance_assessment_strategy.clone()),
                escrow_level: Decimal::percent(5),
                ..Vault::default()
            },
        );

        let response = get_vault_performance_handler(deps.as_ref(), &env, vault.id).unwrap();

        assert_eq!(
            response.fee,
            Coin::new(
                ((standard_received_amount * response.factor - standard_received_amount)
                    * Decimal::percent(20))
                .into(),
                DENOM_STAKE
            )
        );
    }
}
