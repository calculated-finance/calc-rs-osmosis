use crate::{
    helpers::{
        fees::get_dca_plus_performance_fee, price::query_belief_price,
        vault::get_dca_plus_performance_factor,
    },
    msg::DcaPlusPerformanceResponse,
    state::{pairs::find_pair, vaults::get_vault},
};
use cosmwasm_std::{Deps, StdError, StdResult, Uint128};

pub fn get_dca_plus_performance_handler(
    deps: Deps,
    vault_id: Uint128,
) -> StdResult<DcaPlusPerformanceResponse> {
    let vault = get_vault(deps.storage, vault_id)?;

    let pair = find_pair(deps.storage, &vault.denoms())?;

    let current_price = query_belief_price(&deps.querier, &pair, vault.get_swap_denom())?;

    vault.swap_adjustment_strategy.clone().map_or(
        Err(StdError::GenericErr {
            msg: format!("Vault {} is not a DCA Plus vault", vault_id),
        }),
        |_| {
            Ok(DcaPlusPerformanceResponse {
                fee: get_dca_plus_performance_fee(&vault, current_price)?,
                factor: get_dca_plus_performance_factor(&vault, current_price)?,
            })
        },
    )
}

#[cfg(test)]
mod get_dca_plus_performance_tests {
    use super::get_dca_plus_performance_handler;
    use crate::{
        constants::{ONE, TEN},
        tests::{
            helpers::setup_vault,
            mocks::{calc_mock_dependencies, DENOM_STAKE, DENOM_UOSMO},
        },
        types::{swap_adjustment_strategy::SwapAdjustmentStrategy, vault::Vault},
    };
    use cosmwasm_std::{testing::mock_env, Coin, Decimal};

    #[test]
    fn if_not_a_dca_plus_vault_fails() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        let vault = setup_vault(deps.as_mut(), env, Vault::default());

        let err = get_dca_plus_performance_handler(deps.as_ref(), vault.id).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Generic error: Vault 0 is not a DCA Plus vault"
        );
    }

    #[test]
    fn performance_fee_and_factor_match() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        let standard_received_amount = TEN - ONE;

        let swap_adjustment_strategy = SwapAdjustmentStrategy::DcaPlus {
            total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
            standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            standard_dca_received_amount: Coin::new(standard_received_amount.into(), DENOM_STAKE),
            escrowed_balance: Coin::new(TEN.into(), DENOM_STAKE),
            model_id: 30,
            escrow_level: Decimal::percent(5),
        };

        let vault = setup_vault(
            deps.as_mut(),
            env,
            Vault {
                swapped_amount: Coin::new(TEN.into(), DENOM_STAKE),
                received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                swap_adjustment_strategy: Some(swap_adjustment_strategy.clone()),
                ..Vault::default()
            },
        );

        let response = get_dca_plus_performance_handler(deps.as_ref(), vault.id).unwrap();

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
