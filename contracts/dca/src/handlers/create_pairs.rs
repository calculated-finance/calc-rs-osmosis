use crate::{
    error::ContractError,
    helpers::validation::{
        assert_route_has_no_duplicate_entries, assert_route_matches_denoms, assert_route_not_empty,
        assert_sender_is_admin,
    },
    state::pairs::save_pair,
    types::pair::Pair,
};
use cosmwasm_std::{DepsMut, MessageInfo, Response};

pub fn create_pairs_handler(
    deps: DepsMut,
    info: MessageInfo,
    pairs: Vec<Pair>,
) -> Result<Response, ContractError> {
    for pair in pairs.clone() {
        assert_sender_is_admin(deps.storage, info.sender.clone())?;
        assert_route_not_empty(pair.route.clone())?;
        assert_route_has_no_duplicate_entries(pair.route.clone())?;

        assert_route_matches_denoms(&deps.querier, &pair)?;

        save_pair(deps.storage, &pair)?;
    }

    Ok(Response::new()
        .add_attribute("create_pairs", "true")
        .add_attribute("count", format!("{:#?}", pairs.len())))
}
