use crate::{
    msg::DisburseEscrowTasksResponse, state::disburse_escrow_tasks::get_disburse_escrow_tasks,
};
use cosmwasm_std::{Deps, Env, StdResult};

pub fn get_disburse_escrow_tasks_handler(
    deps: Deps,
    env: Env,
    limit: Option<u16>,
) -> StdResult<DisburseEscrowTasksResponse> {
    let tasks = get_disburse_escrow_tasks(deps.storage, env.block.time, limit)?;

    Ok(DisburseEscrowTasksResponse { vault_ids: tasks })
}
