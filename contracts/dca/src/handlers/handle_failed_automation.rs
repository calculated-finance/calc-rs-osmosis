use crate::{
    error::ContractError,
    state::{
        cache::{POST_EXECUTION_ACTION_CACHE, VAULT_CACHE},
        events::create_event,
        vaults::get_vault,
    },
    types::event::{EventBuilder, EventData},
};
use cosmwasm_std::{BankMsg, DepsMut, Env, Reply, Response, SubMsg, SubMsgResult};

pub fn handle_failed_automation_handler(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let vault_id = VAULT_CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, vault_id)?;

    let mut cache = POST_EXECUTION_ACTION_CACHE.load(deps.storage, vault_id.into())?;
    let entry = cache.pop_front().unwrap();
    POST_EXECUTION_ACTION_CACHE.save(deps.storage, vault_id.into(), &cache)?;

    let destination_num = vault.destinations.len() - cache.len();

    Ok(match reply.result {
        SubMsgResult::Ok(_) => Response::new()
            .add_attribute(format!("destination_msg_{}", destination_num), "succeeded"),
        SubMsgResult::Err(_) => {
            create_event(
                deps.storage,
                EventBuilder::new(
                    vault_id,
                    env.block,
                    EventData::DcaVaultPostExecutionActionFailed {
                        msg: entry.msg,
                        funds: entry.funds.clone(),
                    },
                ),
            )?;

            Response::new()
                .add_attribute(format!("destination_msg_{}", destination_num), "failed")
                .add_submessage(SubMsg::new(BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: entry.funds,
                }))
        }
    })
}

#[cfg(test)]
mod handle_failed_automation_handler_tests {
    use super::handle_failed_automation_handler;
    use crate::{
        constants::AFTER_FAILED_AUTOMATION_REPLY_ID,
        handlers::get_events_by_resource_id::get_events_by_resource_id_handler,
        helpers::disbursement::get_disbursement_messages,
        state::cache::{PostExecutionActionCacheEntry, POST_EXECUTION_ACTION_CACHE},
        tests::{
            helpers::{instantiate_contract, setup_vault},
            mocks::ADMIN,
        },
        types::{
            destination::Destination,
            event::{EventBuilder, EventData},
            vault::Vault,
        },
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, Addr, BankMsg, Coin, Decimal, Reply, SubMsg, SubMsgResponse, SubMsgResult,
        WasmMsg,
    };
    use std::collections::VecDeque;

    #[test]
    fn removes_appropriate_post_execution_action_cache_entry_on_success() {
        let mut deps = mock_dependencies();
        let env = mock_env();

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

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                destinations: destinations.clone(),
                ..Vault::default()
            },
        );

        get_disbursement_messages(deps.as_mut().storage, &vault, vault.swap_amount).unwrap();

        handle_failed_automation_handler(
            deps.as_mut(),
            env,
            Reply {
                id: AFTER_FAILED_AUTOMATION_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let cache = POST_EXECUTION_ACTION_CACHE
            .load(deps.as_ref().storage, vault.id.into())
            .unwrap();

        assert_eq!(
            cache,
            VecDeque::from(vec![PostExecutionActionCacheEntry {
                msg: SubMsg::reply_always(
                    WasmMsg::Execute {
                        contract_addr: destinations[1].address.to_string(),
                        msg: destinations[1].msg.clone().unwrap(),
                        funds: vec![Coin::new(
                            (vault.swap_amount * destinations[1].allocation).into(),
                            vault.target_denom.clone()
                        )],
                    },
                    AFTER_FAILED_AUTOMATION_REPLY_ID
                ),
                funds: vec![Coin::new(
                    (vault.swap_amount * destinations[1].allocation).into(),
                    vault.target_denom
                )],
            }])
        );
    }

    #[test]
    fn sends_no_message_on_success() {
        let mut deps = mock_dependencies();
        let env = mock_env();

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

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                destinations: destinations.clone(),
                ..Vault::default()
            },
        );

        get_disbursement_messages(deps.as_mut().storage, &vault, vault.swap_amount).unwrap();

        let response = handle_failed_automation_handler(
            deps.as_mut(),
            env,
            Reply {
                id: AFTER_FAILED_AUTOMATION_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        assert!(response.messages.is_empty());
    }

    #[test]
    fn removes_appropriate_post_execution_action_cache_entry_on_failure() {
        let mut deps = mock_dependencies();
        let env = mock_env();

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

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                destinations: destinations.clone(),
                ..Vault::default()
            },
        );

        get_disbursement_messages(deps.as_mut().storage, &vault, vault.swap_amount).unwrap();

        handle_failed_automation_handler(
            deps.as_mut(),
            env,
            Reply {
                id: AFTER_FAILED_AUTOMATION_REPLY_ID,
                result: SubMsgResult::Err("error".to_string()),
            },
        )
        .unwrap();

        let cache = POST_EXECUTION_ACTION_CACHE
            .load(deps.as_ref().storage, vault.id.into())
            .unwrap();

        assert_eq!(
            cache,
            VecDeque::from(vec![PostExecutionActionCacheEntry {
                msg: SubMsg::reply_always(
                    WasmMsg::Execute {
                        contract_addr: destinations[1].address.to_string(),
                        msg: destinations[1].msg.clone().unwrap(),
                        funds: vec![Coin::new(
                            (vault.swap_amount * destinations[1].allocation).into(),
                            vault.target_denom.clone()
                        )],
                    },
                    AFTER_FAILED_AUTOMATION_REPLY_ID
                ),
                funds: vec![Coin::new(
                    (vault.swap_amount * destinations[1].allocation).into(),
                    vault.target_denom
                )],
            }])
        );
    }

    #[test]
    fn creates_post_execution_action_failed_event_on_failure() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let destinations = vec![
            Destination {
                address: Addr::unchecked("contract1"),
                allocation: Decimal::percent(30),
                msg: Some(
                    to_binary(&WasmMsg::Execute {
                        contract_addr: "contract2".to_string(),
                        msg: to_binary("test").unwrap(),
                        funds: vec![],
                    })
                    .unwrap(),
                ),
            },
            Destination {
                address: Addr::unchecked("contract2"),
                allocation: Decimal::percent(80),
                msg: Some(
                    to_binary(&WasmMsg::Execute {
                        contract_addr: "contract2".to_string(),
                        msg: to_binary("test").unwrap(),
                        funds: vec![],
                    })
                    .unwrap(),
                ),
            },
        ];

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                destinations: destinations.clone(),
                ..Vault::default()
            },
        );

        get_disbursement_messages(deps.as_mut().storage, &vault, vault.swap_amount).unwrap();

        handle_failed_automation_handler(
            deps.as_mut(),
            env.clone(),
            Reply {
                id: AFTER_FAILED_AUTOMATION_REPLY_ID,
                result: SubMsgResult::Err("error".to_string()),
            },
        )
        .unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert_eq!(
            events[0],
            EventBuilder::new(
                vault.id,
                env.block,
                EventData::DcaVaultPostExecutionActionFailed {
                    msg: SubMsg::reply_always(
                        WasmMsg::Execute {
                            contract_addr: destinations[0].address.to_string(),
                            msg: destinations[0].msg.clone().unwrap(),
                            funds: vec![Coin::new(
                                (vault.swap_amount * destinations[0].allocation).into(),
                                vault.target_denom.clone()
                            )]
                        },
                        AFTER_FAILED_AUTOMATION_REPLY_ID
                    ),
                    funds: vec![Coin::new(
                        (vault.swap_amount * destinations[0].allocation).into(),
                        vault.target_denom.clone()
                    )],
                },
            )
            .build(1)
        )
    }

    #[test]
    fn sends_funds_to_vault_owner_on_failure() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let destinations = vec![
            Destination {
                address: Addr::unchecked("contract1"),
                allocation: Decimal::percent(30),
                msg: Some(
                    to_binary(&WasmMsg::Execute {
                        contract_addr: "contract2".to_string(),
                        msg: to_binary("test").unwrap(),
                        funds: vec![],
                    })
                    .unwrap(),
                ),
            },
            Destination {
                address: Addr::unchecked("contract2"),
                allocation: Decimal::percent(80),
                msg: Some(
                    to_binary(&WasmMsg::Execute {
                        contract_addr: "contract2".to_string(),
                        msg: to_binary("test").unwrap(),
                        funds: vec![],
                    })
                    .unwrap(),
                ),
            },
        ];

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                destinations: destinations.clone(),
                ..Vault::default()
            },
        );

        get_disbursement_messages(deps.as_mut().storage, &vault, vault.swap_amount).unwrap();

        let response = handle_failed_automation_handler(
            deps.as_mut(),
            env.clone(),
            Reply {
                id: AFTER_FAILED_AUTOMATION_REPLY_ID,
                result: SubMsgResult::Err("error".to_string()),
            },
        )
        .unwrap();

        assert_eq!(
            response.messages,
            vec![SubMsg::new(BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![Coin::new(
                    (vault.swap_amount * destinations[0].allocation).into(),
                    vault.target_denom.clone()
                )],
            })]
        );
    }
}
