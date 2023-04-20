use crate::types::pair::Pair;
use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::Map;

const PAIRS: Map<String, Pair> = Map::new("pairs_v5");

pub fn save_pair(storage: &mut dyn Storage, pair: &Pair) -> StdResult<()> {
    PAIRS.save(storage, pair.key(), pair)
}

fn key_from(denoms: &[String; 2]) -> String {
    format!("{}-{}", denoms[0], denoms[1])
}

pub fn find_pair(storage: &dyn Storage, denoms: &[String; 2]) -> StdResult<Pair> {
    let pair = PAIRS.may_load(storage, key_from(denoms))?;

    if let Some(pair) = pair {
        return Ok(pair);
    }

    PAIRS.load(storage, key_from(&[denoms[1].clone(), denoms[0].clone()]))
}

pub fn get_pairs(storage: &dyn Storage) -> Vec<Pair> {
    PAIRS
        .range(storage, None, None, Order::Ascending)
        .flat_map(|result| result.map(|(_, pair)| pair))
        .collect::<Vec<Pair>>()
}

#[cfg(test)]
mod pairs_state_tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn saves_and_finds_pair() {
        let mut deps = mock_dependencies();
        let pair = Pair::default();

        save_pair(deps.as_mut().storage, &pair).unwrap();

        let saved_pair = find_pair(&deps.storage, &pair.denoms()).unwrap();
        assert_eq!(pair, saved_pair);
    }

    #[test]
    fn saves_and_finds_pair_with_denoms_reversed() {
        let mut deps = mock_dependencies();
        let pair = Pair::default();

        save_pair(deps.as_mut().storage, &pair).unwrap();

        let denoms = [pair.denoms()[1].clone(), pair.denoms()[0].clone()];

        let saved_pair = find_pair(&deps.storage, &denoms).unwrap();
        assert_eq!(pair, saved_pair);
    }
}
