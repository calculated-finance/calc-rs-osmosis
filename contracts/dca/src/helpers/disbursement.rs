use super::math::checked_mul;
use crate::{
    constants::AFTER_FAILED_AUTOMATION_REPLY_ID,
    state::cache::{PostExecutionActionCacheEntry, POST_EXECUTION_ACTION_CACHE},
    types::vault::Vault,
};
use cosmwasm_std::{BankMsg, Coin, StdResult, Storage, SubMsg, Uint128, WasmMsg};
use std::collections::VecDeque;

pub fn get_disbursement_messages(
    store: &mut dyn Storage,
    vault: &Vault,
    amount_to_disburse: Uint128,
) -> StdResult<VecDeque<SubMsg>> {
    let mut post_execution_action_caches = VecDeque::<PostExecutionActionCacheEntry>::new();

    let messages = vault
        .destinations
        .iter()
        .flat_map(|destination| {
            let allocation_amount = Coin::new(
                checked_mul(amount_to_disburse, destination.allocation)
                    .expect("amount to be distributed should be valid")
                    .into(),
                vault.target_denom.clone(),
            );

            if allocation_amount.amount.gt(&Uint128::zero()) {
                let msg = destination.msg.clone().map_or(
                    SubMsg::reply_always(
                        BankMsg::Send {
                            to_address: destination.address.to_string(),
                            amount: vec![allocation_amount.clone()],
                        },
                        AFTER_FAILED_AUTOMATION_REPLY_ID,
                    ),
                    |msg| {
                        SubMsg::reply_always(
                            WasmMsg::Execute {
                                contract_addr: destination.address.to_string(),
                                msg,
                                funds: vec![allocation_amount.clone()],
                            },
                            AFTER_FAILED_AUTOMATION_REPLY_ID,
                        )
                    },
                );

                post_execution_action_caches.push_back(PostExecutionActionCacheEntry {
                    msg: msg.clone(),
                    funds: vec![allocation_amount],
                });

                return Some(msg);
            }

            None
        })
        .collect::<VecDeque<SubMsg>>();

    POST_EXECUTION_ACTION_CACHE.save(store, vault.id.into(), &post_execution_action_caches)?;

    Ok(messages)
}

#[cfg(test)]
mod get_disbursement_messages_tests {
    use super::get_disbursement_messages;
    use crate::{
        constants::{AFTER_FAILED_AUTOMATION_REPLY_ID, ONE},
        state::cache::POST_EXECUTION_ACTION_CACHE,
        types::{destination::Destination, vault::Vault},
    };
    use cosmwasm_std::{
        testing::mock_dependencies, to_binary, Addr, BankMsg, Coin, Decimal, SubMsg, WasmMsg,
    };

    #[test]
    fn generates_bank_sends_for_destinations_with_no_msg() {
        let mut deps = mock_dependencies();

        let destination = Destination {
            address: Addr::unchecked("test"),
            allocation: Decimal::percent(100),
            msg: None,
        };

        let vault = Vault {
            destinations: vec![destination.clone()],
            ..Vault::default()
        };

        let messages = get_disbursement_messages(deps.as_mut().storage, &vault, ONE).unwrap();

        assert!(messages.contains(&SubMsg::reply_always(
            BankMsg::Send {
                to_address: destination.address.to_string(),
                amount: vec![Coin::new(ONE.into(), vault.target_denom)],
            },
            AFTER_FAILED_AUTOMATION_REPLY_ID
        )))
    }

    #[test]
    fn saves_disbursement_messages_to_cache_queue() {
        let mut deps = mock_dependencies();

        let destinations = vec![
            Destination {
                address: Addr::unchecked("owner"),
                allocation: Decimal::percent(30),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("contract"),
                allocation: Decimal::percent(80),
                msg: Some(
                    to_binary(&WasmMsg::Execute {
                        contract_addr: "contract".to_string(),
                        msg: to_binary("test").unwrap(),
                        funds: vec![],
                    })
                    .unwrap(),
                ),
            },
        ];

        let vault = Vault {
            destinations: destinations.clone(),
            ..Vault::default()
        };

        get_disbursement_messages(deps.as_mut().storage, &vault, ONE).unwrap();

        let mut cache = POST_EXECUTION_ACTION_CACHE
            .load(deps.as_ref().storage, vault.id.into())
            .unwrap();

        assert_eq!(cache.len(), 2);
        assert_eq!(
            cache.pop_front().unwrap().msg,
            SubMsg::reply_always(
                BankMsg::Send {
                    to_address: destinations[0].address.to_string(),
                    amount: vec![Coin::new(
                        (ONE * destinations[0].allocation).into(),
                        vault.target_denom.clone()
                    )],
                },
                AFTER_FAILED_AUTOMATION_REPLY_ID
            )
        );
        assert_eq!(
            cache.pop_front().unwrap().msg,
            SubMsg::reply_always(
                WasmMsg::Execute {
                    contract_addr: destinations[1].address.to_string(),
                    msg: destinations[1].msg.clone().unwrap(),
                    funds: vec![Coin::new(
                        (ONE * destinations[1].allocation).into(),
                        vault.target_denom
                    )],
                },
                AFTER_FAILED_AUTOMATION_REPLY_ID
            )
        );
    }
}
