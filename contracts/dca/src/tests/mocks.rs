use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_slice, to_binary, Binary, ContractResult, CustomQuery, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, StdError, StdResult, SystemError, SystemResult,
};
use osmosis_std::shim::Any;
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::gamm::v1beta1::{Pool, PoolAsset, QueryPoolResponse};
use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceResponse;
use osmosis_std::types::osmosis::poolmanager::v1beta1::EstimateSwapExactAmountInResponse;
use prost::Message;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::constants::{ONE, ONE_DECIMAL, TEN};

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const FEE_COLLECTOR: &str = "fee_collector";
pub const DENOM_UOSMO: &str = "uosmo";
pub const DENOM_STAKE: &str = "stake";

pub struct CalcMockQuerier<C: DeserializeOwned = Empty> {
    default_stargate_handler: Box<dyn for<'a> Fn(&'a str) -> StdResult<Binary>>,
    stargate_handler: Box<dyn for<'a> Fn(&'a str) -> StdResult<Binary>>,
    mock_querier: MockQuerier<C>,
}

impl<C: DeserializeOwned> CalcMockQuerier<C> {
    pub fn new() -> Self {
        Self {
            default_stargate_handler: Box::new(|path| match path {
                "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                    spot_price: ONE_DECIMAL.to_string(),
                }),
                "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                    to_binary(&EstimateSwapExactAmountInResponse {
                        token_out_amount: ONE.to_string(),
                    })
                }
                "/osmosis.gamm.v1beta1.Query/Pool" => to_binary(&QueryPoolResponse {
                    pool: Some(Any {
                        type_url: Pool::TYPE_URL.to_string(),
                        value: Pool {
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
                                        denom: DENOM_STAKE.to_string(),
                                        amount: TEN.to_string(),
                                    }),
                                    weight: TEN.to_string(),
                                },
                            ],
                            ..Pool::default()
                        }
                        .encode_to_vec(),
                    }),
                }),
                _ => panic!("Unexpected path: {}", path),
            }),
            stargate_handler: Box::new(|_| {
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
        WH: Fn(&str) -> StdResult<Binary>,
    {
        self.stargate_handler = Box::from(stargate_handler);
    }

    pub fn handle_query(&self, request: &QueryRequest<C>) -> QuerierResult {
        match &request {
            QueryRequest::Stargate { path, .. } => SystemResult::Ok(ContractResult::Ok(
                (*self.stargate_handler)(path)
                    .unwrap_or_else(|_| (*self.default_stargate_handler)(path).unwrap()),
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
