use crate::{
    state::config::{update_config, Config},
    validation::assert_sender_is_admin,
};
use cosmwasm_std::{DepsMut, MessageInfo, Response, StdResult};

pub fn update_config_handler(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
) -> StdResult<Response> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    deps.api.addr_validate(&config.admin.to_string())?;
    update_config(deps.storage, config)?;
    Ok(Response::new().add_attribute("method", "update_config"))
}

#[cfg(test)]
mod update_config_handler_tests {
    use super::update_config_handler;
    use crate::state::config::{get_config, update_config, Config};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_info},
        Addr,
    };

    #[test]
    fn saves_config() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);

        update_config(
            deps.as_mut().storage,
            Config {
                admin: Addr::unchecked("admin"),
                paused: true,
            },
        )
        .unwrap();

        let old_config = get_config(&deps.storage).unwrap();

        update_config_handler(
            deps.as_mut(),
            info.clone(),
            Config {
                admin: Addr::unchecked("new_admin"),
                paused: true,
            },
        )
        .unwrap();

        let new_config = get_config(&deps.storage).unwrap();

        assert_eq!(old_config.admin, Addr::unchecked("admin"));
        assert_eq!(new_config.admin, Addr::unchecked("new_admin"));
        assert_eq!(old_config.paused, true);
        assert_eq!(new_config.paused, true);
    }
}
