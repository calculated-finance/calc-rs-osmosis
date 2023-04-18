use crate::{
    helpers::{
        fee_helpers::get_dca_plus_performance_fee, price_helpers::query_belief_price,
        vault_helpers::get_dca_plus_performance_factor,
    },
    msg::DcaPlusPerformanceResponse,
    state::vaults::get_vault,
};
use cosmwasm_std::{Deps, StdError, StdResult, Uint128};

pub fn get_dca_plus_performance_handler(
    deps: Deps,
    vault_id: Uint128,
) -> StdResult<DcaPlusPerformanceResponse> {
    let vault = get_vault(deps.storage, vault_id)?;

    let current_price = query_belief_price(&deps.querier, &vault.pair, vault.get_swap_denom())?;

    vault.dca_plus_config.clone().map_or(
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
            helpers::setup_new_vault,
            mocks::{calc_mock_dependencies, DENOM_STAKE},
        },
        types::{dca_plus_config::DcaPlusConfig, vault::Vault},
    };
    use cosmwasm_std::{testing::mock_env, Coin, Decimal};

    #[test]
    fn if_not_a_dca_plus_vault_fails() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        let vault = setup_new_vault(deps.as_mut(), env, Vault::default());

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

        let dca_plus_config = DcaPlusConfig {
            standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_STAKE),
            standard_dca_received_amount: Coin::new(standard_received_amount.into(), DENOM_STAKE),
            escrowed_balance: Coin::new(TEN.into(), DENOM_STAKE),
            ..DcaPlusConfig::default()
        };

        let vault = setup_new_vault(
            deps.as_mut(),
            env,
            Vault {
                swapped_amount: Coin::new(TEN.into(), DENOM_STAKE),
                received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                dca_plus_config: Some(dca_plus_config.clone()),
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
