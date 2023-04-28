use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_slice, to_binary, Binary, ContractResult, CustomQuery, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, StdError, StdResult, SystemError, SystemResult, WasmQuery,
};
use osmosis_std::shim::Any;
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::gamm::v1beta1::{
    Pool, PoolAsset, PoolParams, QueryCalcJoinPoolSharesResponse, QueryPoolRequest,
    QueryPoolResponse,
};
use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceResponse;
use osmosis_std::types::osmosis::poolmanager::v1beta1::EstimateSwapExactAmountInResponse;
use prost::Message;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::constants::{ONE, ONE_DECIMAL, SWAP_FEE_RATE, TEN};

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const FEE_COLLECTOR: &str = "fee_collector";
pub const VALIDATOR: &str = "validator";

pub const DENOM_UOSMO: &str = "uosmo";
pub const DENOM_STAKE: &str = "stake";
pub const DENOM_UATOM: &str = "uatom";
pub const DENOM_UION: &str = "uion";
pub const DENOM_USDC: &str = "uaxlusdc";

pub struct CalcMockQuerier<C: DeserializeOwned = Empty> {
    default_stargate_handler: Box<dyn for<'a> Fn(&'a str, &Binary) -> StdResult<Binary>>,
    stargate_handler: Box<dyn for<'a> Fn(&'a str, &Binary) -> StdResult<Binary>>,
    mock_querier: MockQuerier<C>,
}

impl<C: DeserializeOwned> CalcMockQuerier<C> {
    pub fn new() -> Self {
        Self {
            default_stargate_handler: Box::new(|path, data| match path {
                "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                    spot_price: ONE_DECIMAL.to_string(),
                }),
                "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                    to_binary(&EstimateSwapExactAmountInResponse {
                        token_out_amount: ONE.to_string(),
                    })
                }
                "/osmosis.gamm.v1beta1.Query/Pool" => {
                    let pools = vec![
                        Pool {
                            id: 0,
                            pool_assets: vec![
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_UOSMO.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_UATOM.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                            ],
                            pool_params: Some(PoolParams {
                                swap_fee: "0.001".to_string(),
                                exit_fee: ".01".to_string(),
                                smooth_weight_change_params: None,
                            }),
                            ..Pool::default()
                        },
                        Pool {
                            id: 1,
                            pool_assets: vec![
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_UOSMO.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_UION.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                            ],
                            pool_params: Some(PoolParams {
                                swap_fee: SWAP_FEE_RATE.to_string(),
                                ..PoolParams::default()
                            }),
                            ..Pool::default()
                        },
                        Pool {
                            id: 2,
                            pool_assets: vec![
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_UION.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_USDC.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                            ],
                            pool_params: Some(PoolParams {
                                swap_fee: SWAP_FEE_RATE.to_string(),
                                ..PoolParams::default()
                            }),
                            ..Pool::default()
                        },
                        Pool {
                            id: 3,
                            pool_assets: vec![
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_STAKE.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_UOSMO.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                            ],
                            pool_params: Some(PoolParams {
                                swap_fee: SWAP_FEE_RATE.to_string(),
                                ..PoolParams::default()
                            }),
                            ..Pool::default()
                        },
                        Pool {
                            id: 4,
                            pool_assets: vec![
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_STAKE.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                                PoolAsset {
                                    token: Some(Coin {
                                        denom: DENOM_UION.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                            ],
                            pool_params: Some(PoolParams {
                                swap_fee: SWAP_FEE_RATE.to_string(),
                                ..PoolParams::default()
                            }),
                            ..Pool::default()
                        },
                    ];

                    let pool_id = QueryPoolRequest::decode(data.as_slice()).unwrap().pool_id;

                    to_binary(&QueryPoolResponse {
                        pool: Some(Any {
                            type_url: Pool::TYPE_URL.to_string(),
                            value: pools[pool_id as usize].clone().encode_to_vec(),
                        }),
                    })
                }
                "/osmosis.gamm.v1beta1.Query/CalcJoinPoolShares" => {
                    to_binary(&QueryCalcJoinPoolSharesResponse {
                        share_out_amount: ONE.to_string(),
                        tokens_out: vec![Coin {
                            amount: ONE.into(),
                            denom: DENOM_UOSMO.to_string(),
                        }],
                    })
                }
                _ => panic!("Unexpected path: {}", path),
            }),
            stargate_handler: Box::new(|_, __| {
                Err(StdError::generic_err(
                    "no custom stargate handler, should invoke the default handler",
                ))
            }),
            mock_querier: MockQuerier::<C>::new(&[]),
        }
    }
}

impl<C: CustomQuery + DeserializeOwned> Querier for CalcMockQuerier<C> {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<C> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl<C: CustomQuery + DeserializeOwned> CalcMockQuerier<C> {
    pub fn update_stargate<WH: 'static>(&mut self, stargate_handler: WH)
    where
        WH: Fn(&str, &Binary) -> StdResult<Binary>,
    {
        self.stargate_handler = Box::from(stargate_handler);
    }

    pub fn update_wasm<WH: 'static>(&mut self, wasm_handler: WH)
    where
        WH: Fn(&WasmQuery) -> QuerierResult,
    {
        self.mock_querier.update_wasm(wasm_handler);
    }

    pub fn handle_query(&self, request: &QueryRequest<C>) -> QuerierResult {
        match &request {
            QueryRequest::Stargate { path, data } => SystemResult::Ok(ContractResult::Ok(
                (*self.stargate_handler)(path, data)
                    .unwrap_or_else(|_| (*self.default_stargate_handler)(path, data).unwrap()),
            )),
            _ => self.mock_querier.handle_query(request),
        }
    }
}

pub fn calc_mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CalcMockQuerier, Empty> {
    OwnedDeps {
        storage: MockStorage::new(),
        api: MockApi::default(),
        querier: CalcMockQuerier::new(),
        custom_query_type: PhantomData,
    }
}
