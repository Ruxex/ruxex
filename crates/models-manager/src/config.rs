use genex_protocol::model_types::ModelCatalog;

#[derive(Debug, Clone, Default)]
pub struct ModelsManagerConfig {
    pub model_context_window: Option<i64>,
    pub model_auto_compact_token_limit: Option<i64>,
    pub tool_output_token_limit: Option<usize>,
    pub base_instructions: Option<String>,
    pub model_catalog: Option<ModelCatalog>,
}
