use std::collections::HashSet;

use ruxex_protocol::user_input::UserInput;
use ruxex_skills::ToolMentionKind;
use ruxex_skills::extract_tool_mentions_with_sigil;
use ruxex_skills::plugin_config_name_from_path;
use ruxex_skills::tool_kind_for_path;

use crate::mention_syntax::PLUGIN_TEXT_MENTION_SIGIL;
use crate::mention_syntax::TOOL_MENTION_SIGIL;

use super::PluginCapabilitySummary;

pub(crate) struct CollectedToolMentions {
    pub(crate) paths: HashSet<String>,
}

pub(crate) fn collect_tool_mentions_from_messages(messages: &[String]) -> CollectedToolMentions {
    collect_tool_mentions_from_messages_with_sigil(messages, TOOL_MENTION_SIGIL)
}

fn collect_tool_mentions_from_messages_with_sigil(
    messages: &[String],
    sigil: char,
) -> CollectedToolMentions {
    let mut paths = HashSet::new();
    for message in messages {
        let mentions = extract_tool_mentions_with_sigil(message, sigil);
        paths.extend(mentions.paths().map(str::to_string));
    }
    CollectedToolMentions { paths }
}

/// Collect explicit structured or linked `plugin://...` mentions.
pub(crate) fn collect_explicit_plugin_mentions(
    input: &[UserInput],
    plugins: &[PluginCapabilitySummary],
) -> Vec<PluginCapabilitySummary> {
    if plugins.is_empty() {
        return Vec::new();
    }

    // `config_name` stores the full plugin ID, not its display or mention name.
    let mentioned_plugin_ids = collect_explicit_plugin_ids(input);
    plugins
        .iter()
        .filter(|plugin| mentioned_plugin_ids.contains(plugin.config_name.as_str()))
        .cloned()
        .collect()
}

/// Collect exact IDs from explicit `plugin://` references, independently of display names.
///
/// Host plugins use `<plugin>@<marketplace>` IDs; selected roots may supply opaque IDs.
pub(crate) fn collect_explicit_plugin_ids(input: &[UserInput]) -> HashSet<String> {
    let messages = input
        .iter()
        .filter_map(|item| match item {
            UserInput::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<String>>();

    input
        .iter()
        .filter_map(|item| match item {
            UserInput::Mention { path, .. } => Some(path.clone()),
            _ => None,
        })
        .chain(
            // Plugin plaintext links use `@`, not the default `$` tool sigil.
            collect_tool_mentions_from_messages_with_sigil(&messages, PLUGIN_TEXT_MENTION_SIGIL)
                .paths,
        )
        .filter(|path| tool_kind_for_path(path.as_str()) == ToolMentionKind::Plugin)
        .filter_map(|path| plugin_config_name_from_path(path.as_str()).map(str::to_string))
        .collect()
}
