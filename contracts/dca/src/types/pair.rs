use super::position_type::PositionType;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct Pair {
    pub address: Addr,
    pub base_denom: String,
    pub quote_denom: String,
    pub route: Vec<u64>,
}

impl Pair {
    pub fn position_type(&self, swap_denom: String) -> PositionType {
        if self.quote_denom == swap_denom {
            PositionType::Enter
        } else {
            PositionType::Exit
        }
    }
}
