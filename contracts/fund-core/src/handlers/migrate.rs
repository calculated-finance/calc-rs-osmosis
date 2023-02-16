use base::ContractError;
use cosmwasm_std::{Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128};

use crate::validation::assert_sender_is_router;

pub fn migrate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_fund_address: Addr,
) -> Result<Response, ContractError> {
    assert_sender_is_router(deps.storage, info.sender.clone())?;

    let mut response = Response::new().add_attribute("method", "migrate");

    let balance: Vec<Coin> = deps
        .querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|coin| coin.amount.gt(&Uint128::zero()))
        .collect();

    if !balance.is_empty() {
        response = response.add_message(BankMsg::Send {
            to_address: new_fund_address.to_string(),
            amount: balance,
        });
    }
    Ok(response)
}
