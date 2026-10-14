use ruxex_protocol::ResponseUsageMetadata;
use ruxex_protocol::models::ResponseItem;
use ruxex_protocol::protocol::ModelVerification;
use ruxex_protocol::protocol::RateLimitSnapshot;
use ruxex_protocol::protocol::TokenUsage;

#[derive(Debug, Clone)]
pub struct ProviderSafetyBuffering {
    pub use_cases: Vec<String>,
    pub reasons: Vec<String>,
    pub show_buffering_ui: bool,
    pub faster_model: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProviderResponseEvent {
    Created {
        response_id: Option<String>,
    },
    SafetyBuffering(ProviderSafetyBuffering),
    OutputItemDone(ResponseItem),
    OutputItemAdded(ResponseItem),
    ServerModel(String),
    ModelVerifications(Vec<ModelVerification>),
    ServerReasoningIncluded(bool),
    Completed {
        response_id: String,
        token_usage: Option<TokenUsage>,
        usage_metadata: Option<ResponseUsageMetadata>,
        end_turn: Option<bool>,
    },
    OutputTextDelta(String),
    ToolCallInputDelta {
        item_id: String,
        call_id: Option<String>,
        delta: String,
    },
    ReasoningSummaryDelta {
        delta: String,
        summary_index: i64,
    },
    ReasoningSummaryDone {
        item_id: String,
        text: String,
        summary_index: i64,
    },
    ReasoningContentDelta {
        delta: String,
        content_index: i64,
    },
    ReasoningSummaryPartAdded {
        summary_index: i64,
    },
    RateLimits(RateLimitSnapshot),
    ModelsEtag(String),
}

#[cfg(test)]
mod tests {
    use super::ProviderResponseEvent;

    fn accepts_provider_neutral_event(_event: &ProviderResponseEvent) {}

    #[test]
    fn telemetry_event_boundary_is_provider_neutral() {
        let event = ProviderResponseEvent::OutputTextDelta("text".to_string());
        accepts_provider_neutral_event(&event);
    }
}
