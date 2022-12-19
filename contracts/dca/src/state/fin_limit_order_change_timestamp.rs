use cosmwasm_std::Timestamp;
use cw_storage_plus::Item;

pub const FIN_LIMIT_ORDER_CHANGE_TIMESTAMP: Item<Timestamp> =
    Item::new("fin_limit_order_change_timestamp");
