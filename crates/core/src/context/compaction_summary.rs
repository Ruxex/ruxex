use super::ContextualUserFragment;
use genex_protocol::models::ContentItemKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompactionSummary {
    summary: String,
}

impl CompactionSummary {
    pub(crate) fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
        }
    }
}

impl ContextualUserFragment for CompactionSummary {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("compaction.summary".to_string())
    }

    fn role(&self) -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("", "")
    }

    fn body(&self) -> String {
        self.summary.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_the_summary_text() {
        let fragment = CompactionSummary::new("Earlier context was compacted.");

        assert_eq!(
            fragment.content_kind(),
            ContentItemKind("compaction.summary".into())
        );
        assert_eq!(fragment.role(), "user");
        assert_eq!(fragment.markers(), ("", ""));
        assert_eq!(fragment.body(), "Earlier context was compacted.");
    }
}
