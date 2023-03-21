use crate::types::pair::Pair;
use crate::{state::config::Config, types::callback::Callback};
use base::pair::Pair as FinPair;
use base::price_type::PriceType;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin, Decimal};

#[cw_serde]
pub struct MigrateMsg {
    pub admin: Addr,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        admin: Addr,
        paused: bool,
    },
    AddPath {
        pair: Pair,
    },
    CreateSwap {
        target_denom: String,
        slippage_tolerance: Option<Decimal>,
        on_complete: Option<Callback>,
    },
    ContinueSwap {
        swap_id: u64,
    },
    SwapOnFin {
        pair: FinPair,
        slippage_tolerance: Option<Decimal>,
        callback: Binary,
    },
    SendFunds {
        address: Addr,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    GetConfig {},
    #[returns(Vec<Vec<Pair>>)]
    GetPaths {
        swap_amount: Coin,
        target_denom: String,
    },
    #[returns(Decimal)]
    GetPrice {
        swap_amount: Coin,
        target_denom: String,
        price_type: PriceType,
    },
}

#[cw_serde]
pub struct PricesResponse {
    pub prices: Vec<Decimal>,
}
