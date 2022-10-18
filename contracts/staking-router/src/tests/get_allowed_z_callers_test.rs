use cosmwasm_std::Addr;

use crate::msg::QueryMsg;

use super::mocks::MockApp;

#[test]
fn should_succeed() {
    let mock = MockApp::new();

    let allowed_z_callers_response: Vec<Addr> = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.staking_router_contract_address,
            &QueryMsg::GetAllowedZCallers {},
        )
        .unwrap();

    assert_eq!(allowed_z_callers_response[0].to_string(), "allowedzcaller")
}
