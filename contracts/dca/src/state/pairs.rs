use base::pair::Pair;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub const PAIRS: Map<Addr, Pair> = Map::new("pairs_v1");
