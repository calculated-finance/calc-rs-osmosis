use crate::{
    error::ContractError,
    helpers::validation_helpers::assert_sender_is_admin_or_vault_owner,
    helpers::{
        disbursement_helpers::{get_disbursement_messages, get_fee_messages},
        vault_helpers::get_dca_plus_fee,
    },
    state::vaults::{get_vault, update_vault},
};
use base::price_type::PriceType;
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response, Uint128};
use fin_helpers::queries::query_price;

pub fn claim_escrowed_funds_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let mut vault = get_vault(deps.storage, vault_id)?;

    assert_sender_is_admin_or_vault_owner(deps.storage, vault.owner.clone(), info.sender)?;

    if vault.dca_plus_config.is_none() {
        return Err(ContractError::CustomError {
            val: "Vault is not a DCA+ vault".to_string(),
        });
    }

    let mut dca_plus_config = vault.dca_plus_config.clone().unwrap();

    let current_price = query_price(
        deps.querier,
        vault.pair.clone(),
        &Coin {
            denom: vault.get_swap_denom(),
            amount: Uint128::one(),
        },
        PriceType::Belief,
    )?;

    let performance_fee = get_dca_plus_fee(&vault, current_price)?;
    let amount_to_disburse = dca_plus_config.escrowed_balance - performance_fee.amount;

    dca_plus_config.escrowed_balance = Uint128::zero();
    vault.dca_plus_config = Some(dca_plus_config);
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
        )?))
}
