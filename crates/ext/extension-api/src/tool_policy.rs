//! A startup ceiling on the tools a thread may advertise or execute.

use crate::ToolName;

/// Supply through `ExtensionDataInit` before starting a thread. The host captures
/// the policy once; changing extension state later cannot widen the tool set.
/// Callers must supply it again when resuming a thread.
#[derive(Clone, Debug, Default)]
pub struct ToolPolicy {
    /// `None` keeps ordinary tool setup; an empty list permits no tools.
    /// Names include their namespace; plain names use the default namespace.
    pub allowed_tools: Option<Vec<ToolName>>,
}

impl ToolPolicy {
    pub fn allows(&self, tool: &ToolName) -> bool {
        self.allowed_tools.as_ref().is_none_or(|allowed_tools| {
            allowed_tools.iter().any(|allowed| {
                allowed.name == tool.name
                    && (allowed.namespace == tool.namespace
                        || (allowed.is_default_namespace() && tool.is_default_namespace()))
            })
        })
    }
}
