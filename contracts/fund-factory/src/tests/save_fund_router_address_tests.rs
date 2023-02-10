use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Event, Reply, SubMsgResponse, SubMsgResult,
};

use crate::{
    contract::{query, AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID},
    handlers::save_fund_router_address::save_fund_router_address,
    msg::{FundRoutersResponse, QueryMsg},
    state::cache::{Cache, CACHE},
    tests::helpers::{instantiate_contract, USER},
};

#[test]
fn saves_fund_router_address() {
    let mut mock_deps = mock_dependencies();
    let mock_env = mock_env();
    let mock_info = mock_info(USER, &vec![]);

    instantiate_contract(mock_deps.as_mut(), mock_env.clone(), mock_info.clone());

    CACHE
        .save(
            mock_deps.as_mut().storage,
            &Cache {
                owner: Addr::unchecked(USER),
            },
        )
        .unwrap();

    save_fund_router_address(
        mock_deps.as_mut(),
        Reply {
            id: AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: None,
                events: vec![Event::new("instantiate").add_attribute("_contract_address", "test")],
            }),
        },
    )
    .unwrap();

    let get_managed_funds_by_address_msg = QueryMsg::GetFundRouters {
        owner: Addr::unchecked(USER),
    };

    let binary = query(
        mock_deps.as_ref(),
        mock_env,
        get_managed_funds_by_address_msg,
    )
    .unwrap();

    let res: FundRoutersResponse = from_binary(&binary).unwrap();

    assert_eq!(res.fund_routers[0], Addr::unchecked("test"));
}

#[test]
fn saves_multiple_fund_router_addresses() {
    let mut mock_deps = mock_dependencies();
    let mock_env = mock_env();
    let mock_info = mock_info(USER, &vec![]);

    instantiate_contract(mock_deps.as_mut(), mock_env.clone(), mock_info.clone());

    CACHE
        .save(
            mock_deps.as_mut().storage,
            &Cache {
                owner: Addr::unchecked(USER),
            },
        )
        .unwrap();

    save_fund_router_address(
        mock_deps.as_mut(),
        Reply {
            id: AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: None,
                events: vec![Event::new("instantiate").add_attribute("_contract_address", "test")],
            }),
        },
    )
    .unwrap();

    save_fund_router_address(
        mock_deps.as_mut(),
        Reply {
            id: AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: None,
                events: vec![Event::new("instantiate").add_attribute("_contract_address", "test2")],
            }),
        },
    )
    .unwrap();

    let get_managed_funds_by_address_msg = QueryMsg::GetFundRouters {
        owner: Addr::unchecked(USER),
    };

    let binary = query(
        mock_deps.as_ref(),
        mock_env,
        get_managed_funds_by_address_msg,
    )
    .unwrap();

    let res: FundRoutersResponse = from_binary(&binary).unwrap();

    assert_eq!(res.fund_routers[0], Addr::unchecked("test"));
    assert_eq!(res.fund_routers[1], Addr::unchecked("test2"));
}
