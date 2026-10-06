use super::ContextualUserFragment;
use genex_protocol::models::ContentItemKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RolloutBudgetContext {
    pub(crate) remaining_tokens: i64,
}

impl ContextualUserFragment for RolloutBudgetContext {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("rollout_budget.remaining_tokens".to_string())
    }

    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("<rollout_budget>\n", "\n</rollout_budget>")
    }

    fn body(&self) -> String {
        format!(
            "You have {} weighted tokens left in the shared session token budget.",
            self.remaining_tokens
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_remaining_tokens_in_the_developer_fragment() {
        let fragment = RolloutBudgetContext {
            remaining_tokens: 1_024,
        };

        assert_eq!(
            fragment.content_kind(),
            ContentItemKind("rollout_budget.remaining_tokens".into())
        );
        assert_eq!(fragment.role(), "developer");
        assert_eq!(
            fragment.markers(),
            ("<rollout_budget>\n", "\n</rollout_budget>")
        );
        assert_eq!(
            fragment.body(),
            "You have 1024 weighted tokens left in the shared session token budget."
        );
    }
}
