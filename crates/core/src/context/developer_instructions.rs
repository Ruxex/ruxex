use super::ContextualUserFragment;
use ruxex_protocol::models::ContentItemKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeveloperInstructions {
    instructions: String,
}

impl DeveloperInstructions {
    pub(crate) fn new(instructions: impl Into<String>) -> Self {
        Self {
            instructions: instructions.into(),
        }
    }
}

impl ContextualUserFragment for DeveloperInstructions {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("generic.developer_instructions".to_string())
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
        self.instructions.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_developer_instructions() {
        let fragment = DeveloperInstructions::new("Use the stable API.");

        assert_eq!(
            fragment.content_kind(),
            ContentItemKind("generic.developer_instructions".into())
        );
        assert_eq!(fragment.role(), "developer");
        assert_eq!(fragment.markers(), ("", ""));
        assert_eq!(fragment.body(), "Use the stable API.");
    }
}
