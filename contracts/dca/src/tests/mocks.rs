









use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_slice, Binary, ContractResult, CustomQuery, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult,
};


use serde::de::DeserializeOwned;

use std::marker::PhantomData;


pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const FEE_COLLECTOR: &str = "fee_collector";
pub const DENOM_UOSMO: &str = "uosmo";
pub const DENOM_STAKE: &str = "stake";

pub struct CalcMockQuerier<C: DeserializeOwned = Empty> {
    stargate_handler: Box<dyn for<'a> Fn(&'a QueryRequest<C>) -> Binary>,
    mock_querier: MockQuerier<C>,
}

impl<C: DeserializeOwned> CalcMockQuerier<C> {
    pub fn new() -> Self {
        Self {
            stargate_handler: Box::new(|_| {
                panic!("This should never be called. Use the update_stargate method to set it")
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
        WH: Fn(&QueryRequest<C>) -> Binary,
    {
        self.stargate_handler = Box::from(stargate_handler);
    }

    pub fn handle_query(&self, request: &QueryRequest<C>) -> QuerierResult {
        match &request {
            QueryRequest::Stargate { .. } => {
                SystemResult::Ok(ContractResult::Ok((*self.stargate_handler)(request)))
            }
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
