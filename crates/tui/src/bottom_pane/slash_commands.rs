//! Shared helpers for filtering and matching built-in and model service-tier slash commands.
//!
//! The same sandbox- and feature-gating rules are used by both the composer
//! and the command popup. Centralizing them here keeps those call sites small
//! and ensures they stay in sync.
use std::str::FromStr;

use genex_utils_fuzzy_match::fuzzy_match;

use crate::slash_command::SlashCommand;
use crate::slash_command::built_in_slash_commands;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceTierCommand {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SlashCommandItem {
    Builtin(SlashCommand),
    ServiceTier(ServiceTierCommand),
}

impl SlashCommandItem {
    pub(crate) fn command(&self) -> &str {
        match self {
            Self::Builtin(cmd) => cmd.command(),
            Self::ServiceTier(command) => &command.name,
        }
    }

    pub(crate) fn supports_inline_args(&self) -> bool {
        match self {
            Self::Builtin(cmd) => cmd.supports_inline_args(),
            Self::ServiceTier(_) => false,
        }
    }

    pub(crate) fn available_in_side_conversation(&self) -> bool {
        match self {
            Self::Builtin(cmd) => cmd.available_in_side_conversation(),
            Self::ServiceTier(_) => false,
        }
    }

    pub(crate) fn available_during_task(&self) -> bool {
        match self {
            Self::Builtin(cmd) => cmd.available_during_task(),
            Self::ServiceTier(_) => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BuiltinCommandFlags {
    pub(crate) collaboration_modes_enabled: bool,
    pub(crate) plugins_command_enabled: bool,
    pub(crate) token_activity_command_enabled: bool,
    pub(crate) service_tier_commands_enabled: bool,
    pub(crate) goal_command_enabled: bool,
    pub(crate) worktrees_enabled: bool,
    pub(crate) side_conversation_active: bool,
}

/// Return the built-ins that should be visible/usable for the current input.
pub(crate) fn builtins_for_input(flags: BuiltinCommandFlags) -> Vec<(&'static str, SlashCommand)> {
    built_in_slash_commands()
        .into_iter()
        .filter(|(_, cmd)| flags.collaboration_modes_enabled || *cmd != SlashCommand::Plan)
        .filter(|(_, cmd)| flags.plugins_command_enabled || *cmd != SlashCommand::Plugins)
        .filter(|(_, cmd)| flags.token_activity_command_enabled || *cmd != SlashCommand::Usage)
        .filter(|(_, cmd)| flags.goal_command_enabled || *cmd != SlashCommand::Goal)
        .filter(|(_, cmd)| flags.worktrees_enabled || *cmd != SlashCommand::Worktree)
        .filter(|(_, cmd)| !flags.side_conversation_active || cmd.available_in_side_conversation())
        .collect()
}

pub(crate) fn commands_for_input(
    flags: BuiltinCommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> Vec<SlashCommandItem> {
    let mut commands = Vec::new();
    let tiers_enabled = flags.service_tier_commands_enabled;
    for (_, cmd) in builtins_for_input(flags) {
        commands.push(SlashCommandItem::Builtin(cmd));
        if cmd == SlashCommand::Model && tiers_enabled {
            commands.extend(
                service_tier_commands
                    .iter()
                    .cloned()
                    .map(SlashCommandItem::ServiceTier),
            );
        }
    }
    commands
        .into_iter()
        .filter(|cmd| !flags.side_conversation_active || cmd.available_in_side_conversation())
        .collect()
}

/// Find a single built-in command by a recognized name or alias, after applying feature gating.
///
/// Side-conversation and token-activity gating are intentionally enforced by dispatch rather than
/// command lookup so a typed command can produce a specific unavailable message while the popup
/// still hides it.
pub(crate) fn find_builtin_command(name: &str, flags: BuiltinCommandFlags) -> Option<SlashCommand> {
    let cmd = SlashCommand::from_str(name).ok().or_else(|| {
        let repeated_os = name.strip_prefix('g')?.strip_suffix("al")?;
        (!repeated_os.is_empty() && repeated_os.bytes().all(|byte| byte == b'o'))
            .then_some(SlashCommand::Goal)
    })?;
    builtins_for_input(BuiltinCommandFlags {
        token_activity_command_enabled: true,
        side_conversation_active: false,
        ..flags
    })
    .into_iter()
    .any(|(_, visible_cmd)| visible_cmd == cmd)
    .then_some(cmd)
}

pub(crate) fn find_slash_command(
    name: &str,
    flags: BuiltinCommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> Option<SlashCommandItem> {
    if let Some(cmd) = find_builtin_command(name, flags) {
        return Some(SlashCommandItem::Builtin(cmd));
    }

    let tiers_enabled = flags.service_tier_commands_enabled;
    tiers_enabled
        .then(|| {
            service_tier_commands
                .iter()
                .find(|command| command.name == name)
                .cloned()
                .map(SlashCommandItem::ServiceTier)
        })
        .flatten()
}

pub(crate) fn has_slash_command_prefix(
    name: &str,
    flags: BuiltinCommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> bool {
    commands_for_input(flags, service_tier_commands)
        .into_iter()
        .any(|command| fuzzy_match(command.command(), name).is_some())
}
