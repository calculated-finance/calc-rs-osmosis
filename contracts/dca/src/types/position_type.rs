use cosmwasm_schema::cw_serde;

#[cw_serde]
#[derive(Hash)]
pub enum PositionType {
    Enter,
    Exit,
}
