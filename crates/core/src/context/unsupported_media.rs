use super::ContextualUserFragment;
use ruxex_protocol::models::ContentItemKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnsupportedMedia {
    text: &'static str,
    content_kind: &'static str,
}

impl UnsupportedMedia {
    pub(crate) const IMAGE: Self = Self {
        text: "image content omitted because you do not support image input",
        content_kind: "images.unsupported",
    };

    pub(crate) const AUDIO: Self = Self {
        text: "audio content omitted because you do not support audio input",
        content_kind: "audio.unsupported",
    };
}

impl ContextualUserFragment for UnsupportedMedia {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind(self.content_kind.to_string())
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
        self.text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_unsupported_image_and_audio_input() {
        assert_eq!(UnsupportedMedia::IMAGE.role(), "user");
        assert_eq!(UnsupportedMedia::IMAGE.markers(), ("", ""));
        assert_eq!(
            UnsupportedMedia::IMAGE.content_kind(),
            ContentItemKind("images.unsupported".into())
        );
        assert_eq!(
            UnsupportedMedia::IMAGE.body(),
            "image content omitted because you do not support image input"
        );

        assert_eq!(UnsupportedMedia::AUDIO.role(), "user");
        assert_eq!(UnsupportedMedia::AUDIO.markers(), ("", ""));
        assert_eq!(
            UnsupportedMedia::AUDIO.content_kind(),
            ContentItemKind("audio.unsupported".into())
        );
        assert_eq!(
            UnsupportedMedia::AUDIO.body(),
            "audio content omitted because you do not support audio input"
        );
    }
}
