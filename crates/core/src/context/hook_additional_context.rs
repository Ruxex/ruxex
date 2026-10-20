use super::ContextualUserFragment;
use ruxex_protocol::models::ContentItemKind;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HookAdditionalContext {
    text: String,
}

impl HookAdditionalContext {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl ContextualUserFragment for HookAdditionalContext {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("hooks.additional_context".to_string())
    }

    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("", "")
    }

    fn body(&self) -> String {
        self.text.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_hook_context_as_developer_content() {
        let fragment = HookAdditionalContext::new("A hook supplied this context.");

        assert_eq!(
            fragment.content_kind(),
            ContentItemKind("hooks.additional_context".into())
        );
        assert_eq!(fragment.role(), "developer");
        assert_eq!(fragment.markers(), ("", ""));
        assert_eq!(fragment.body(), "A hook supplied this context.");
    }
}
