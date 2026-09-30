use std::{collections::HashSet, error::Error, io, time::Duration};

use alloy::{
    primitives::Address,
    signers::{Signer, local::PrivateKeySigner},
};
use rust_decimal::Decimal;
use stockfront_adapters::glider::GliderAdapter;
use stockfront_core::{
    baskets::BasketKind,
    bstock_market::BNB_CHAIN_ID,
    managed_portfolios::{
        EnrollManagedPortfolioRequest, ListManagedPortfolioTemplatesQuery,
        ManagedPortfolioCollection, ManagedPortfolioGateway, ManagedPortfolioSort,
        PrepareManagedPortfolioEnrollmentRequest,
    },
    wallet_portfolio::WalletAddress,
};
use zeroize::Zeroizing;

const ENROLLMENT_CONFIRMATION: &str = "I_UNDERSTAND_THIS_CREATES_A_MANAGED_PORTFOLIO";

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn discovers_live_bnb_managed_portfolio_templates() -> Result<(), Box<dyn Error>> {
    let reader = live_gateway()?;
    let query = ListManagedPortfolioTemplatesQuery::new(
        Some(ManagedPortfolioCollection::Curated),
        None,
        Some(50),
        None,
    )?;

    let page = reader.templates(&query).await?;

    assert!(
        !page.items.is_empty(),
        "Glider returned no curated BNB templates"
    );
    for template in page.items {
        assert!(!template.id.is_empty());
        assert_eq!(template.kind, BasketKind::ManagedPortfolio);
        assert_eq!(template.provider_id, "glider");
        assert_eq!(template.provider_name, "Glider");
        assert!(!template.name.is_empty());
        assert_eq!(template.name, template.name.trim());
        assert!(
            template
                .description
                .as_ref()
                .is_none_or(|description| !description.is_empty())
        );
        assert!(
            template
                .advertised_max_apy_percent
                .is_none_or(|apy| apy >= Decimal::ZERO)
        );
        assert!(!template.allocation.is_empty());
        assert_eq!(
            template
                .allocation
                .iter()
                .map(|asset| asset.weight_percent)
                .sum::<Decimal>(),
            Decimal::ONE_HUNDRED
        );
        for asset in template.allocation {
            assert_eq!(asset.chain_id, BNB_CHAIN_ID);
            assert_ne!(asset.contract, alloy_primitives::Address::ZERO);
            assert!(asset.weight_percent > Decimal::ZERO);
        }
        assert!(
            template
                .metrics
                .tvl_usd
                .is_none_or(|tvl| tvl >= Decimal::ZERO)
        );
        for performance in template.metrics.performance {
            assert!(!performance.window.is_empty());
            assert!(!performance.since.is_empty());
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn follows_live_managed_portfolio_template_cursor() -> Result<(), Box<dyn Error>> {
    let reader = live_gateway()?;
    let first = reader
        .templates(&ListManagedPortfolioTemplatesQuery::new(
            Some(ManagedPortfolioCollection::TopPerforming),
            Some(ManagedPortfolioSort::Tvl),
            Some(5),
            None,
        )?)
        .await?;
    let cursor = first
        .next_cursor
        .ok_or("Glider did not return a cursor for a partial ranked page")?;
    let second = reader
        .templates(&ListManagedPortfolioTemplatesQuery::new(
            Some(ManagedPortfolioCollection::TopPerforming),
            Some(ManagedPortfolioSort::Tvl),
            Some(5),
            Some(cursor),
        )?)
        .await?;

    assert!(!first.items.is_empty() || !second.items.is_empty());
    let first_ids = first
        .items
        .iter()
        .map(|template| template.id.as_str())
        .collect::<HashSet<_>>();
    assert!(
        second
            .items
            .iter()
            .all(|template| !first_ids.contains(template.id.as_str()))
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires live Glider API credentials"]
async fn prepares_live_bnb_managed_portfolio_enrollment() -> Result<(), Box<dyn Error>> {
    let gateway = live_gateway()?;
    let prepared = gateway
        .prepare_enrollment(&PrepareManagedPortfolioEnrollmentRequest {
            wallet: test_wallet()?,
            template_id: std::env::var("STOCKFRONT_GLIDER_TEMPLATE_ID")?,
        })
        .await?;

    assert!(!prepared.authorization.flow_id.is_empty());
    assert!(!prepared.authorization.account_index.is_empty());
    assert!(!prepared.authorization.agent_account_id.is_empty());
    assert_ne!(
        prepared.personal_sign_message,
        alloy::primitives::B256::ZERO
    );
    Ok(())
}

#[tokio::test]
#[ignore = "creates a live Glider managed portfolio"]
async fn enrolls_live_bnb_managed_portfolio() -> Result<(), Box<dyn Error>> {
    if std::env::var("STOCKFRONT_GLIDER_ENROLL_CONFIRM")
        .ok()
        .as_deref()
        != Some(ENROLLMENT_CONFIRMATION)
    {
        return Err(io::Error::other(format!(
            "set STOCKFRONT_GLIDER_ENROLL_CONFIRM={ENROLLMENT_CONFIRMATION} to create a portfolio"
        ))
        .into());
    }

    let gateway = live_gateway()?;
    let private_key = Zeroizing::new(std::env::var("STOCKFRONT_E2E_PRIVATE_KEY")?);
    let signer = private_key.parse::<PrivateKeySigner>()?;
    drop(private_key);
    let wallet = WalletAddress::try_from(signer.address())?;
    let template_id = std::env::var("STOCKFRONT_GLIDER_TEMPLATE_ID")?;
    let prepared = gateway
        .prepare_enrollment(&PrepareManagedPortfolioEnrollmentRequest {
            wallet,
            template_id: template_id.clone(),
        })
        .await?;
    let signature = signer
        .sign_message(prepared.personal_sign_message.as_slice())
        .await?
        .to_string();
    let enrollment = gateway
        .enroll(&EnrollManagedPortfolioRequest {
            wallet,
            template_id: template_id.clone(),
            authorization: prepared.authorization,
            signature,
            name: std::env::var("STOCKFRONT_GLIDER_PORTFOLIO_NAME").ok(),
        })
        .await?;

    assert!(!enrollment.portfolio_id.is_empty());
    assert_eq!(enrollment.template_id, template_id);
    assert_eq!(enrollment.smart_account.chain_id, BNB_CHAIN_ID);
    assert_ne!(enrollment.smart_account.address, Address::ZERO);
    Ok(())
}

fn live_gateway() -> Result<GliderAdapter, Box<dyn Error>> {
    let api_key = std::env::var("GLIDER_API_KEY")?;
    let base_url = std::env::var("GLIDER_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.glider.fi/v2".to_owned());
    Ok(GliderAdapter::new(
        &base_url,
        api_key,
        Duration::from_secs(10),
    )?)
}

fn test_wallet() -> Result<WalletAddress, Box<dyn Error>> {
    Ok(WalletAddress::try_from(
        std::env::var("STOCKFRONT_TEST_WALLET")?.parse::<Address>()?,
    )?)
}
