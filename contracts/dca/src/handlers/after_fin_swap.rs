use crate::error::ContractError;
use crate::helpers::disbursement_helpers::get_disbursement_messages;
use crate::helpers::fee_helpers::{get_delegation_fee_rate, get_fee_messages, get_swap_fee_rate};
use crate::helpers::vault_helpers::get_swap_amount;
use crate::msg::ExecuteMsg;
use crate::state::cache::{CACHE, SWAP_CACHE};
use crate::state::events::create_event;
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::types::dca_plus_config::DcaPlusConfig;
use base::events::event::{EventBuilder, EventData, ExecutionSkippedReason};
use base::helpers::coin_helpers::add_to_coin;
use base::helpers::math_helpers::checked_mul;
use base::vaults::vault::VaultStatus;
use cosmwasm_std::{to_binary, CosmosMsg, Decimal, SubMsg, SubMsgResult, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Attribute, Coin, DepsMut, Env, Reply, Response};

pub fn after_fin_swap(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let mut vault = get_vault(deps.storage, cache.vault_id.into())?;

    let mut attributes: Vec<Attribute> = Vec::new();
    let mut sub_msgs: Vec<SubMsg> = Vec::new();

    match reply.result {
        SubMsgResult::Ok(_) => {
            let swap_cache = SWAP_CACHE.load(deps.storage)?;

            let swap_denom_balance = &deps
                .querier
                .query_balance(&env.contract.address, &vault.get_swap_denom())?;

            let receive_denom_balance = &deps
                .querier
                .query_balance(&env.contract.address, &vault.get_receive_denom())?;

            let coin_sent = Coin::new(
                (swap_cache.swap_denom_balance.amount - swap_denom_balance.amount).into(),
                swap_denom_balance.denom.clone(),
            );

            let coin_received = Coin::new(
                (receive_denom_balance.amount - swap_cache.receive_denom_balance.amount).into(),
                receive_denom_balance.denom.clone(),
            );

            let swap_fee_rate = match vault.dca_plus_config {
                Some(_) => Decimal::zero(),
                None => get_swap_fee_rate(&deps, &vault)?,
            };

            let automation_fee_rate = match vault.dca_plus_config {
                Some(_) => Decimal::zero(),
                None => get_delegation_fee_rate(&deps, &vault)?,
            };

            let swap_fee = checked_mul(coin_received.amount, swap_fee_rate)?;
            let total_after_swap_fee = coin_received.amount - swap_fee;
            let automation_fee = checked_mul(total_after_swap_fee, automation_fee_rate)?;
            let total_fee = swap_fee + automation_fee;
            let mut total_after_total_fee = coin_received.amount - total_fee;

            sub_msgs.append(&mut get_fee_messages(
                deps.as_ref(),
                env.clone(),
                vec![swap_fee, automation_fee],
                coin_received.denom.clone(),
                false,
            )?);

            vault.balance.amount -= get_swap_amount(&deps.as_ref(), &env, vault.clone())?.amount;
            vault.swapped_amount = add_to_coin(vault.swapped_amount, coin_sent.amount);
            vault.received_amount = add_to_coin(vault.received_amount, total_after_total_fee);

            if let Some(dca_plus_config) = vault.dca_plus_config.clone() {
                let amount_to_escrow = total_after_total_fee * dca_plus_config.escrow_level;
                total_after_total_fee -= amount_to_escrow;

                vault.dca_plus_config = Some(DcaPlusConfig {
                    escrowed_balance: add_to_coin(
                        dca_plus_config.escrowed_balance,
                        amount_to_escrow,
                    ),
                    ..dca_plus_config
                });
            }

            if vault.balance.amount.is_zero() {
                vault.status = VaultStatus::Inactive;
            }

            sub_msgs.append(&mut get_disbursement_messages(
                deps.as_ref(),
                &vault,
                total_after_total_fee,
            )?);

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.clone(),
                    EventData::DcaVaultExecutionCompleted {
                        sent: coin_sent.clone(),
                        received: coin_received.clone(),
                        fee: Coin::new(total_fee.into(), coin_received.denom.clone()),
                    },
                ),
            )?;

            attributes.push(Attribute::new("status", "success"));
        }
        SubMsgResult::Err(_) => {
            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.to_owned(),
                    EventData::DcaVaultExecutionSkipped {
                        reason: match vault.has_sufficient_funds() {
                            true => ExecutionSkippedReason::SlippageToleranceExceeded,
                            false => ExecutionSkippedReason::UnknownFailure,
                        },
                    },
                ),
            )?;

            if !vault.has_sufficient_funds() {
                vault.status = VaultStatus::Inactive;
            }

            attributes.push(Attribute::new("status", "skipped"));
        }
    }

    update_vault(deps.storage, &vault)?;

    if vault.is_finished_dca_plus_vault() {
        sub_msgs.push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id })?,
            funds: vec![],
        })));

        delete_trigger(deps.storage, vault.id)?;
    }

    Ok(Response::new()
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attributes(attributes)
        .add_submessages(sub_msgs))
}
