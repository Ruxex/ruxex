/// Identifies privileged Ruxex approvals. Changing this key or adding another
/// privileged discriminator requires updating the MCP elicitation router's
/// form-forwarding safeguards.
pub const APPROVAL_KIND_KEY: &str = "ruxex_approval_kind";
pub const APPROVAL_KIND_MCP_TOOL_CALL: &str = "mcp_tool_call";
pub const APPROVAL_KIND_BROWSER_AUTH: &str = "browser_auth";
/// Marks requests that need user input even when their form schema is empty.
pub const REQUIRES_USER_INPUT_KEY: &str = "ruxex_requires_user_input";
pub const REQUEST_TYPE_KEY: &str = "ruxex_request_type";
pub const REQUEST_TYPE_APPROVAL_REQUEST: &str = "approval_request";
/// Marks a sensitive action requiring synchronous review when routed to automatic review.
/// Absent or false preserves the existing approval path; this does not opt into review.
pub const SENSITIVE_ACTION_KEY: &str = "ruxex_sensitive_action";
pub const PERSIST_KEY: &str = "persist";
pub const PERSIST_SESSION: &str = "session";
pub const PERSIST_ALWAYS: &str = "always";
pub const SOURCE_KEY: &str = "source";
pub const SOURCE_CONNECTOR: &str = "connector";
pub const CONNECTOR_ID_KEY: &str = "connector_id";
pub const CONNECTOR_NAME_KEY: &str = "connector_name";
pub const CONNECTOR_DESCRIPTION_KEY: &str = "connector_description";
pub const TOOL_TITLE_KEY: &str = "tool_title";
pub const TOOL_DESCRIPTION_KEY: &str = "tool_description";
pub const TOOL_PARAMS_KEY: &str = "tool_params";
pub const TOOL_PARAMS_DISPLAY_KEY: &str = "tool_params_display";
