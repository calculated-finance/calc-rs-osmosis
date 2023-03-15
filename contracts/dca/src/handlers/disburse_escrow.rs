use crate::{
    error::ContractError,
    helpers::{
        disbursement_helpers::get_disbursement_messages,
        fee_helpers::{get_dca_plus_performance_fee, get_fee_messages},
        validation_helpers::assert_sender_is_contract_or_admin,
    },
    state::vaults::{get_vault, update_vault},
    types::dca_plus_config::DcaPlusConfig,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use fin_helpers::queries::query_belief_price;

pub fn disburse_escrow_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    assert_sender_is_contract_or_admin(deps.storage, &info.sender, &env)?;

    let mut vault = get_vault(deps.storage, vault_id)?;

    if vault.dca_plus_config.is_none() {
        return Err(ContractError::CustomError {
            val: "Vault is not a DCA+ vault".to_string(),
        });
    }

    let dca_plus_config = vault.dca_plus_config.clone().unwrap();

    let current_price =
        query_belief_price(deps.querier, vault.pair.clone(), &vault.get_swap_denom())?;

    let performance_fee = get_dca_plus_performance_fee(&vault, current_price)?;
    let amount_to_disburse = dca_plus_config.escrowed_balance - performance_fee.amount;

    vault.dca_plus_config = Some(DcaPlusConfig {
        escrowed_balance: Uint128::zero(),
        ..dca_plus_config
    });

    update_vault(deps.storage, &vault)?;

    Ok(Response::new()
        .add_submessages(get_disbursement_messages(
            deps.as_ref(),
            &vault,
            amount_to_disburse,
        )?)
        .add_submessages(get_fee_messages(
            deps.as_ref(),
            env,
            vec![performance_fee.amount],
            vault.get_receive_denom(),
            true,
        )?)
        .add_attribute("performance_fee", format!("{:?}", performance_fee))
        .add_attribute("escrow_disbursed", format!("{:?}", amount_to_disburse)))
}
