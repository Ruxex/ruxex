//! Per-response usage metadata reported by the upstream service, without aggregation.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

/// Usage metadata reported for one upstream response.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, TS, JsonSchema)]
pub struct ResponseUsageMetadata {
    /// Provider-reported amount consumed by this completed API response.
    /// Ruxex aggregates this approximate floating-point value without assuming
    /// a currency or provider-independent unit.
    #[serde(default, deserialize_with = "deserialize_amount")]
    pub amount: Option<f64>,
    pub metadata: Option<Value>,
}

fn deserialize_amount<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Amount {
        Number(f64),
        String(String),
    }

    let amount = Option::<Amount>::deserialize(deserializer)?;
    let amount = match amount {
        Some(Amount::Number(value)) => Some(value),
        Some(Amount::String(value)) => {
            Some(value.parse::<f64>().map_err(serde::de::Error::custom)?)
        }
        None => None,
    };
    if amount.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(serde::de::Error::custom(
            "amount must be finite and non-negative",
        ));
    }
    Ok(amount)
}
