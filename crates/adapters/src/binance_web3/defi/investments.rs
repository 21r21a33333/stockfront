use std::{collections::HashSet, sync::Arc};

use alloy_primitives::Address;
use binance_web3_rwa::{
    apis::de_fi_data_api::{
        GetDefiInvestmentDetailParams, ListDefiInvestmentsParams, get_defi_investment_detail,
        list_defi_investments,
    },
    models,
};
use futures_util::{StreamExt, TryStreamExt, stream};

use super::super::{
    client::{BinanceWeb3Client, InvestmentListKind},
    provider::{
        BinanceRequestError, InvalidProviderData, ProviderError, ensure_business_success, mapped,
        required, retried,
    },
};

pub(super) const INVESTMENT_PAGE_SIZE: i32 = 100;
/// How many product details are read at once.
///
/// A listing fans out into one detail request per product, and reading them one
/// at a time is what made earn routes take seconds. The bound keeps the burst
/// small enough that the provider answers it rather than rate-limiting it.
const DETAIL_CONCURRENCY: usize = 12;

pub(crate) struct ProviderInvestmentDetail {
    pub(crate) investment: models::DefiInvestmentDetail,
    pub(crate) observed_at_ms: i64,
}

impl BinanceWeb3Client {
    pub(super) async fn investment_ids<E, F>(
        &self,
        kind: InvestmentListKind,
        asset: Address,
        request: F,
    ) -> Result<Arc<Vec<String>>, E>
    where
        E: From<BinanceRequestError> + From<InvalidProviderData>,
        F: FnMut(i32) -> models::ListDefiInvestmentsRequest,
    {
        self.investment_lists
            .get_or_load((kind, asset), async {
                self.read_investment_ids(request).await.map(Arc::new)
            })
            .await
            .map_err(mapped)
    }

    async fn read_investment_ids<F>(&self, mut request: F) -> Result<Vec<String>, ProviderError>
    where
        F: FnMut(i32) -> models::ListDefiInvestmentsRequest,
    {
        let mut page_number = 1;
        let mut investment_ids = Vec::new();
        let mut seen = HashSet::new();
        let mut provider_record_count = 0_usize;

        loop {
            let body = request(page_number);
            let response = retried(|| {
                list_defi_investments(
                    &self.configuration,
                    ListDefiInvestmentsParams {
                        list_defi_investments_request: body.clone(),
                    },
                )
            })
            .await?;
            ensure_business_success(response.code, response.success)?;
            let page = required(response.data)?;
            let total = page.total;
            let investments = page.list.unwrap_or_default();
            let record_count = investments.len();
            provider_record_count += record_count;
            append_investment_ids(&mut investment_ids, &mut seen, investments);
            if !has_next_page(page_number, record_count, total) {
                break;
            }
            page_number += 1;
        }

        if provider_record_count > 0 && investment_ids.is_empty() {
            Err(InvalidProviderData.into())
        } else {
            Ok(investment_ids)
        }
    }

    pub(super) async fn investment_detail<E>(
        &self,
        investment_id: &str,
    ) -> Result<Option<Arc<ProviderInvestmentDetail>>, E>
    where
        E: From<BinanceRequestError> + From<InvalidProviderData>,
    {
        self.cached_investment_detail(investment_id)
            .await
            .map_err(mapped)
    }

    /// Reads several product details concurrently, preserving the requested order.
    pub(super) async fn investment_details<E>(
        &self,
        investment_ids: &[String],
    ) -> Result<Vec<Option<Arc<ProviderInvestmentDetail>>>, E>
    where
        E: From<BinanceRequestError> + From<InvalidProviderData>,
    {
        let reads = investment_ids
            .iter()
            .map(|investment_id| self.cached_investment_detail(investment_id))
            .collect::<Vec<_>>();
        stream::iter(reads)
            .buffered(DETAIL_CONCURRENCY)
            .try_collect::<Vec<_>>()
            .await
            .map_err(mapped)
    }

    async fn cached_investment_detail(
        &self,
        investment_id: &str,
    ) -> Result<Option<Arc<ProviderInvestmentDetail>>, ProviderError> {
        self.investment_details
            .get_or_load(investment_id.to_owned(), async {
                self.read_investment_detail(investment_id)
                    .await
                    .map(|detail| detail.map(Arc::new))
            })
            .await
    }

    async fn read_investment_detail(
        &self,
        investment_id: &str,
    ) -> Result<Option<ProviderInvestmentDetail>, ProviderError> {
        let body = models::GetDefiInvestmentDetailRequest::new(investment_id.to_owned());
        let response = retried(|| {
            get_defi_investment_detail(
                &self.configuration,
                GetDefiInvestmentDetailParams {
                    get_defi_investment_detail_request: body.clone(),
                },
            )
        })
        .await?;
        match ensure_business_success(response.code, response.success) {
            Ok(()) => {}
            Err(BinanceRequestError::Rejected) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        Ok(Some(ProviderInvestmentDetail {
            investment: *required(response.data)?,
            observed_at_ms: required(response.timestamp)?,
        }))
    }
}

fn append_investment_ids(
    investment_ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
    investments: Vec<models::DefiInvestment>,
) {
    for investment_id in investments
        .into_iter()
        .filter_map(|investment| investment.investment_id)
        .filter(|investment_id| !investment_id.is_empty())
    {
        if seen.insert(investment_id.clone()) {
            investment_ids.push(investment_id);
        }
    }
}

fn has_next_page(page: i32, record_count: usize, total: Option<i64>) -> bool {
    if let Some(total) = total {
        return i64::from(page) * i64::from(INVESTMENT_PAGE_SIZE) < total;
    }
    record_count == INVESTMENT_PAGE_SIZE as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_deduplicates_investment_ids_and_honors_provider_totals() {
        let investments = ["first", "first", "second"]
            .into_iter()
            .map(|id| {
                let mut investment = models::DefiInvestment::new();
                investment.investment_id = Some(id.to_owned());
                investment
            })
            .collect();
        let mut investment_ids = Vec::new();
        let mut seen = HashSet::new();

        append_investment_ids(&mut investment_ids, &mut seen, investments);

        assert_eq!(investment_ids, vec!["first", "second"]);
        assert!(has_next_page(1, INVESTMENT_PAGE_SIZE as usize, Some(101)));
        assert!(has_next_page(1, 1, Some(101)));
        assert!(!has_next_page(2, 1, Some(101)));
    }
}
