use super::ContextualUserFragment;
use ruxex_protocol::models::ContentItemKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BaseInstructionsFragment(pub(crate) String);

impl ContextualUserFragment for BaseInstructionsFragment {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("model.base_instructions".to_string())
    }

    fn role(&self) -> &'static str {
        "developer"
    }

    fn requires_separate_message(&self) -> bool {
        true
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("", "")
    }

    fn body(&self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_as_a_separate_developer_message() {
        let fragment = BaseInstructionsFragment("Follow the repository instructions.".into());

        assert_eq!(
            fragment.content_kind(),
            ContentItemKind("model.base_instructions".into())
        );
        assert_eq!(fragment.role(), "developer");
        assert!(fragment.requires_separate_message());
        assert_eq!(fragment.markers(), ("", ""));
        assert_eq!(fragment.body(), "Follow the repository instructions.");
    }
}
