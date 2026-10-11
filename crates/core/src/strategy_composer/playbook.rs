//! The strategies Stockfront offers as products.
//!
//! One definition, so the model drafting a strategy and the cards offering one
//! cannot disagree about what is on offer. The chain is written in the same
//! grammar the model drafts in, which is what lets it rebuild any of these
//! against a different stock or basket than the one a card happens to show.

/// Where a strategy sits against the others here. It is a comparison between
/// these, never a claim that one is safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HouseStrategyRisk {
    Lower,
    High,
    NoLeverage,
}

impl HouseStrategyRisk {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lower => "lower risk",
            Self::High => "high risk",
            Self::NoLeverage => "no leverage, market risk only",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HouseStrategy {
    pub name: &'static str,
    pub risk: HouseStrategyRisk,
    /// The chain in the model's own grammar.
    pub chain: &'static str,
    /// What the user has to want for this to be the right answer.
    pub when: &'static str,
}

pub const HOUSE_STRATEGIES: [HouseStrategy; 3] = [
    HouseStrategy {
        name: "Borrow & earn",
        risk: HouseStrategyRisk::Lower,
        chain: "WALLET_ASSET(stablecoin) -> TRADE(stock) -> SUPPLY -> BORROW(stablecoin) -> EARN_DEPOSIT(that stock)",
        when: "income on top of holding a stock: yield, earning on idle stablecoins, or a safe or conservative return. Start from SUPPLIED_POSITION instead when the user already supplied the stock.",
    },
    HouseStrategy {
        name: "Leveraged stock",
        risk: HouseStrategyRisk::High,
        chain: "WALLET_ASSET(stablecoin) -> TRADE(stock) -> SUPPLY -> BORROW(stablecoin) -> TRADE(the same stock) -> SUPPLY",
        when: "more exposure to one stock than the cash alone buys: the highest return, going long, or leverage, where the user accepts the risk.",
    },
    HouseStrategy {
        name: "Index basket",
        risk: HouseStrategyRisk::NoLeverage,
        chain: "WALLET_ASSET(stablecoin) -> TRADE(basket)",
        when: "broad or themed exposure without picking one company: a fund, an index, diversification, or a risk-adjusted holding.",
    },
];
