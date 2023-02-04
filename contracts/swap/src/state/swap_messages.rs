use super::state::fetch_and_increment_counter;
use crate::types::callback::Callback;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map};
use std::collections::VecDeque;

const SWAP_MESSAGES_COUNTER: Item<u64> = Item::new("swap_messages_counter_v1");

const SWAP_MESSAGES: Map<u64, VecDeque<Callback>> = Map::new("swap_messages_v1");

pub fn get_next_swap_id(store: &mut dyn Storage) -> StdResult<u64> {
    fetch_and_increment_counter(store, SWAP_MESSAGES_COUNTER)
}

pub fn save_swap_messages(
    store: &mut dyn Storage,
    swap_id: u64,
    messages: VecDeque<Callback>,
) -> StdResult<()> {
    SWAP_MESSAGES.save(store, swap_id, &messages)
}

pub fn get_swap_messages(store: &dyn Storage, swap_id: u64) -> StdResult<VecDeque<Callback>> {
    SWAP_MESSAGES.load(store, swap_id)
}
