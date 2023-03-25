use base::pool::Pool;
use cw_storage_plus::Map;

pub const POOLS: Map<u64, Pool> = Map::new("pools_v1");
