use std::collections::BTreeMap;
use std::fmt;

use ruxex_model_provider::{ProviderFuture, ProviderResult};

/// Provider-specific credentials completed by a client-side auth flow.
pub trait ProviderCredentials: Send + Sync + 'static {
    const PROVIDER_ID: &'static str;

    fn into_fields(self) -> BTreeMap<String, String>;
}

/// Initial values supplied when a credential flow starts.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct CredentialInput {
    pub values: BTreeMap<String, String>,
}

impl fmt::Debug for CredentialInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialInput")
            .field("values", &"<redacted>")
            .finish()
    }
}

impl CredentialInput {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// A value submitted to the current credential step.
pub enum CredentialValue {
    Text(String),
    Selection(String),
}

impl CredentialValue {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn secret(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn selection(value: impl Into<String>) -> Self {
        Self::Selection(value.into())
    }

    pub fn into_string(self) -> String {
        match self {
            Self::Text(value) | Self::Selection(value) => value,
        }
    }
}

impl fmt::Debug for CredentialValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::Text(_) => "Text",
            Self::Selection(_) => "Selection",
        };
        f.debug_tuple(kind).field(&"<redacted>").finish()
    }
}

/// An option displayed by a selection step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialOption {
    pub value: String,
    pub label: String,
}

impl CredentialOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// A UI-independent instruction emitted by a credential flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CredentialStep {
    Input {
        id: String,
        label: String,
        secret: bool,
    },
    Select {
        id: String,
        label: String,
        options: Vec<CredentialOption>,
    },
    Info {
        message: String,
    },
    Complete,
}

impl CredentialStep {
    pub fn input(
        id: impl Into<String>,
        label: impl Into<String>,
        secret: bool,
    ) -> Self {
        Self::Input {
            id: id.into(),
            label: label.into(),
            secret,
        }
    }

    pub fn select(
        id: impl Into<String>,
        label: impl Into<String>,
        options: Vec<CredentialOption>,
    ) -> Self {
        Self::Select {
            id: id.into(),
            label: label.into(),
            options,
        }
    }
}

/// Stateful client-side credential collection flow.
pub trait CredentialFlow<C: ProviderCredentials>: Send {
    fn current_step(&self) -> CredentialStep;

    fn submit(
        &mut self,
        value: CredentialValue,
    ) -> ProviderFuture<'_, ProviderResult<CredentialStep>>;

    fn finish(self: Box<Self>) -> ProviderResult<C>;
}

/// Provider-specific client-side credential discovery contract.
pub trait ProviderCredentialResolver<C: ProviderCredentials>: Send + Sync {
    fn begin(
        &self,
        input: CredentialInput,
    ) -> ProviderFuture<'_, ProviderResult<Box<dyn CredentialFlow<C>>>>;
}

/// Typed provider login boundary used when composing a Provider and Resolver.
pub trait ProviderCredentialSink<C: ProviderCredentials>: Send + Sync {
    fn login_typed(
        &self,
        credentials: C,
    ) -> ProviderFuture<'_, ProviderResult<()>>;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{CredentialStep, CredentialValue, ProviderCredentials};

    #[derive(Debug, PartialEq, Eq)]
    struct TestCredentials {
        api_key: String,
        tenant_id: String,
    }

    impl ProviderCredentials for TestCredentials {
        const PROVIDER_ID: &'static str = "test";

        fn into_fields(self) -> BTreeMap<String, String> {
            BTreeMap::from([
                ("api_key".to_string(), self.api_key),
                ("tenant_id".to_string(), self.tenant_id),
            ])
        }
    }

    #[test]
    fn credentials_convert_to_fields_only_at_the_final_boundary() {
        let fields = TestCredentials {
            api_key: "secret".to_string(),
            tenant_id: "tenant-a".to_string(),
        }
        .into_fields();

        assert_eq!(fields["api_key"], "secret");
        assert_eq!(fields["tenant_id"], "tenant-a");
    }

    #[test]
    fn flow_steps_represent_input_selection_and_completion() {
        let input = CredentialStep::input("api_key", "API key", true);
        let select = CredentialStep::select(
            "tenant_id",
            "Tenant",
            vec![super::CredentialOption::new("tenant-a", "Tenant A")],
        );

        assert!(matches!(input, CredentialStep::Input { secret: true, .. }));
        assert!(matches!(select, CredentialStep::Select { .. }));
        assert_eq!(CredentialStep::Complete, CredentialStep::Complete);
    }

    #[test]
    fn secret_values_are_redacted_from_debug_output() {
        let value = CredentialValue::secret("do-not-print");
        let step = CredentialStep::input("api_key", "API key", true);

        assert!(!format!("{value:?}").contains("do-not-print"));
        assert!(!format!("{step:?}").contains("do-not-print"));
    }
}
