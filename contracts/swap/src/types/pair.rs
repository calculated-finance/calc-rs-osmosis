use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub enum Pair {
    Fin {
        address: Addr,
        quote_denom: String,
        base_denom: String,
    },
}

impl Pair {
    pub fn get_denoms(&self) -> [String; 2] {
        match self {
            Pair::Fin {
                base_denom,
                quote_denom,
                ..
            } => [base_denom.clone(), quote_denom.clone()],
        }
    }
}
