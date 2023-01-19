use cosmwasm_std::{from_binary, to_binary, Binary, StdResult, Storage};
use cw_storage_plus::Item;
use petgraph::{algo::astar, Graph};

use crate::types::pair::Pair;

const PATHS: Item<Binary> = Item::new("paths_v1");

pub fn add_path(store: &mut dyn Storage, denoms: [String; 2], pair: Pair) -> StdResult<()> {
    let mut graph: Graph<String, Pair> = from_binary(
        &PATHS
            .load(store)
            .unwrap_or(to_binary(&Graph::<String, Pair>::new()).expect("empty paths graph")),
    )?;
    let denom_1 = graph
        .node_indices()
        .find(|node| graph[*node] == denoms[0])
        .unwrap_or_else(|| graph.add_node(denoms[0].clone()));
    let denom_2 = graph
        .node_indices()
        .find(|node| graph[*node] == denoms[1])
        .unwrap_or_else(|| graph.add_node(denoms[1].clone()));
    graph.add_edge(denom_1, denom_2, pair);
    PATHS.save(store, &to_binary(&graph)?)?;
    Ok(())
}

pub fn get_path(store: &dyn Storage, denoms: [String; 2]) -> StdResult<Vec<Pair>> {
    let graph: Graph<String, Pair> = from_binary(&PATHS.load(store)?)?;
    let denom_1 = graph.node_indices().find(|node| graph[*node] == denoms[0]);
    let denom_2 = graph.node_indices().find(|node| graph[*node] == denoms[1]);
    Ok(if let (Some(node_a), Some(node_b)) = (denom_1, denom_2) {
        astar(&graph, node_a, |n| n == node_b, |_| 0, |_| 0)
            .map(|p| {
                p.1.windows(2)
                    .map(|nodes| {
                        graph.find_edge(nodes[0], nodes[1]).expect(&format!(
                            "path from {} to {}",
                            nodes[0].index(),
                            nodes[1].index()
                        ))
                    })
                    .map(|edge| graph[edge].clone())
                    .collect::<Vec<Pair>>()
            })
            .unwrap_or(vec![])
    } else {
        vec![]
    })
}

#[cfg(test)]
mod path_tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn add_path_adds_nodes_and_edge() {
        let mut deps = mock_dependencies();
        let graph = Graph::<String, Pair>::new();
        PATHS
            .save(deps.as_mut().storage, &to_binary(&graph).unwrap())
            .unwrap();
        add_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_b".to_string()],
            Pair::Fin {
                address: "address".to_string(),
            },
        )
        .unwrap();
        let graph: Graph<String, Pair> = from_binary(&PATHS.load(&deps.storage).unwrap()).unwrap();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn get_path_returns_empty_if_no_path() {
        let mut deps = mock_dependencies();
        let graph = Graph::<String, Pair>::new();
        PATHS
            .save(deps.as_mut().storage, &to_binary(&graph).unwrap())
            .unwrap();
        let path = get_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_b".to_string()],
        )
        .unwrap();
        assert_eq!(path, vec![]);
    }

    #[test]
    fn get_path_returns_path_if_path_exists() {
        let mut deps = mock_dependencies();
        let graph = Graph::<String, Pair>::new();
        PATHS
            .save(deps.as_mut().storage, &to_binary(&graph).unwrap())
            .unwrap();
        add_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_b".to_string()],
            Pair::Fin {
                address: "address".to_string(),
            },
        )
        .unwrap();
        let path = get_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_b".to_string()],
        )
        .unwrap();
        assert_eq!(
            path,
            vec![Pair::Fin {
                address: "address".to_string()
            }]
        );
    }

    #[test]
    fn get_path_returns_empty_if_path_does_not_exist() {
        let mut deps = mock_dependencies();
        let graph = Graph::<String, Pair>::new();
        PATHS
            .save(deps.as_mut().storage, &to_binary(&graph).unwrap())
            .unwrap();
        add_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_b".to_string()],
            Pair::Fin {
                address: "address_1".to_string(),
            },
        )
        .unwrap();
        add_path(
            deps.as_mut().storage,
            ["denom_c".to_string(), "denom_d".to_string()],
            Pair::Fin {
                address: "address_2".to_string(),
            },
        )
        .unwrap();
        let path = get_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_c".to_string()],
        )
        .unwrap();
        assert_eq!(path, vec![]);
    }

    #[test]
    fn get_path_returns_path_if_multihop_path_exists() {
        let mut deps = mock_dependencies();
        let graph = Graph::<String, Pair>::new();
        PATHS
            .save(deps.as_mut().storage, &to_binary(&graph).unwrap())
            .unwrap();
        add_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_b".to_string()],
            Pair::Fin {
                address: "address_1".to_string(),
            },
        )
        .unwrap();
        add_path(
            deps.as_mut().storage,
            ["denom_b".to_string(), "denom_c".to_string()],
            Pair::Fin {
                address: "address_2".to_string(),
            },
        )
        .unwrap();
        let path = get_path(
            deps.as_mut().storage,
            ["denom_a".to_string(), "denom_c".to_string()],
        )
        .unwrap();
        assert_eq!(
            path,
            vec![
                Pair::Fin {
                    address: "address_1".to_string()
                },
                Pair::Fin {
                    address: "address_2".to_string()
                }
            ]
        );
    }
}
