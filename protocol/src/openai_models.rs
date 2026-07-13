use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;

/// See https://platform.openai.com/docs/guides/reasoning?api-mode=responses#get-started-with-reasoning
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Hash, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    XHigh,
    Max,
}

#[cfg(test)]
mod tests {
    use super::ReasoningEffort;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn max_reasoning_effort_round_trips_as_lowercase_max() {
        let serialized = serde_json::to_value(ReasoningEffort::Max).expect("serialize effort");
        assert_eq!(serialized, json!("max"));

        let parsed: ReasoningEffort = serde_json::from_value(json!("max")).expect("parse effort");
        assert_eq!(parsed, ReasoningEffort::Max);
    }
}
