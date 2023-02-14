use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Event, Reply, SubMsgResponse, SubMsgResult,
};

use crate::{
    contract::{query, AFTER_INSTANTIATE_ROUTER_REPLY_ID},
    handlers::save_router::save_router_handler,
    msg::{RoutersResponse, QueryMsg},
    state::cache::{Cache, CACHE},
    tests::helpers::{instantiate_contract, USER},
};

#[test]
fn saves_router_address() {
    let mut mock_deps = mock_dependencies();
    let mock_env = mock_env();
    let mock_info = mock_info(USER, &vec![]);

    instantiate_contract(mock_deps.as_mut(), mock_env.clone(), mock_info.clone());

    CACHE
        .save(
            mock_deps.as_mut().storage,
            &Cache {
                owner: Addr::unchecked(USER),
                router_address: None,
            },
        )
        .unwrap();

    save_router_handler(
        mock_deps.as_mut(),
        Reply {
            id: AFTER_INSTANTIATE_ROUTER_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: None,
                events: vec![Event::new("instantiate").add_attribute("_contract_address", "test")],
            }),
        },
    )
    .unwrap();

    let get_routers_by_address_msg = QueryMsg::GetRouters {
        owner: Addr::unchecked(USER),
    };

    let binary = query(
        mock_deps.as_ref(),
        mock_env,
        get_routers_by_address_msg,
    )
    .unwrap();

    let res: RoutersResponse = from_binary(&binary).unwrap();

    assert_eq!(res.routers[0], Addr::unchecked("test"));
}

#[test]
fn saves_multiple_router_addresses() {
    let mut mock_deps = mock_dependencies();
    let mock_env = mock_env();
    let mock_info = mock_info(USER, &vec![]);

    instantiate_contract(mock_deps.as_mut(), mock_env.clone(), mock_info.clone());

    CACHE
        .save(
            mock_deps.as_mut().storage,
            &Cache {
                owner: Addr::unchecked(USER),
                router_address: None,
            },
        )
        .unwrap();

    save_router_handler(
        mock_deps.as_mut(),
        Reply {
            id: AFTER_INSTANTIATE_ROUTER_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: None,
                events: vec![Event::new("instantiate").add_attribute("_contract_address", "test")],
            }),
        },
    )
    .unwrap();

    save_router_handler(
        mock_deps.as_mut(),
        Reply {
            id: AFTER_INSTANTIATE_ROUTER_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: None,
                events: vec![Event::new("instantiate").add_attribute("_contract_address", "test2")],
            }),
        },
    )
    .unwrap();

    let get_routers_by_address_msg = QueryMsg::GetRouters {
        owner: Addr::unchecked(USER),
    };

    let binary = query(
        mock_deps.as_ref(),
        mock_env,
        get_routers_by_address_msg,
    )
    .unwrap();

    let res: RoutersResponse = from_binary(&binary).unwrap();

    assert_eq!(res.routers[0], Addr::unchecked("test"));
    assert_eq!(res.routers[1], Addr::unchecked("test2"));
}
