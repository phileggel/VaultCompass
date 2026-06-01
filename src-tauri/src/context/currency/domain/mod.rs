/// CurrencyPair aggregate, repository trait, and CurrencyPairSummary.
pub mod currency_pair;
/// CurrencyRate aggregate, repository trait, and CurrencyRateSource.
pub mod currency_rate;

pub use currency_pair::{CurrencyPair, CurrencyPairRepository, CurrencyPairSummary};
pub use currency_rate::{CurrencyRate, CurrencyRateRepository, CurrencyRateSource};

#[cfg(test)]
pub use currency_pair::MockCurrencyPairRepository;
#[cfg(test)]
pub use currency_rate::MockCurrencyRateRepository;
