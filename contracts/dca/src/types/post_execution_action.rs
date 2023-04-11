use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum PostExecutionAction {
    Send,
    ZDelegate,
    ZProvideLiquidity {
        pool_id: u64,
        // duration: Duration
    },
}
