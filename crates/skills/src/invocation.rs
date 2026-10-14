use std::path::Path;

use ruxex_protocol::parse_command::ParsedCommand;
use ruxex_shell_command::parse_command::parse_command_impl;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_path_uri::PathUri;

use crate::SkillMetadata;

/// A skill document read or script execution identified in a shell command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImplicitSkillAccess {
    Document(PathUri),
    Script(PathUri),
}

/// Provides the indexed skill lookups used to recognize implicit invocations.
pub trait ImplicitSkillLookup {
    fn implicit_skill_for_scripts_dir(&self, path: &AbsolutePathBuf) -> Option<&SkillMetadata>;

    fn implicit_skill_for_doc_path(&self, path: &AbsolutePathBuf) -> Option<&SkillMetadata>;
}

pub fn detect_implicit_skill_invocation_for_command(
    outcome: &impl ImplicitSkillLookup,
    command: &str,
    workdir: &AbsolutePathBuf,
) -> Option<SkillMetadata> {
    let workdir = canonicalize_if_exists(workdir);
    let tokens = tokenize_command(command);

    if let Some(candidate) = detect_skill_script_run(outcome, tokens.as_slice(), &workdir) {
        return Some(candidate);
    }

    detect_skill_doc_read(outcome, tokens.as_slice(), &workdir)
}

/// Resolves statically recognizable skill accesses without consulting the host filesystem.
pub fn implicit_skill_accesses_for_command(
    command: &str,
    workdir: &PathUri,
) -> Vec<ImplicitSkillAccess> {
    let tokens = tokenize_command(command);
    let mut accesses = Vec::new();
    if let Some(path) = powershell_doc_path(&tokens)
        && let Some(path) = powershell_path_to_uri(path, workdir)
    {
        accesses.push(ImplicitSkillAccess::Document(path));
    }
    if let Some(script) = script_run_token(&tokens)
        && let Ok(path) = workdir.join(script)
    {
        accesses.push(ImplicitSkillAccess::Script(path));
    }

    for parsed in parse_command_impl(&tokens) {
        if let ParsedCommand::Read { path, .. } = parsed
            && let Some(path) = path.to_str()
            && let Ok(path) = workdir.join(path)
        {
            accesses.push(ImplicitSkillAccess::Document(path));
        }
    }

    accesses
}

fn tokenize_command(command: &str) -> Vec<String> {
    if command
        .split_whitespace()
        .next()
        .map(command_basename)
        .is_some_and(|command| {
            matches!(
                command.to_ascii_lowercase().as_str(),
                "get-content" | "gc" | "type"
            )
        })
    {
        return tokenize_powershell(command);
    }

    shlex::split(command)
        .unwrap_or_else(|| command.split_whitespace().map(str::to_string).collect())
}

fn tokenize_powershell(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quote = None;

    for character in command.chars() {
        match quote {
            Some(delimiter) if character == delimiter => quote = None,
            Some(_) => token.push(character),
            None if character == '\'' || character == '"' => quote = Some(character),
            None if character.is_whitespace() => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            None => token.push(character),
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

fn script_run_token(tokens: &[String]) -> Option<&str> {
    const RUNNERS: [&str; 9] = [
        "python", "python3", "bash", "zsh", "sh", "node", "deno", "ruby", "perl",
    ];
    const SCRIPT_EXTENSIONS: [&str; 6] = [".py", ".sh", ".js", ".ts", ".rb", ".pl"];

    let runner_token = tokens.first()?;
    let runner = command_basename(runner_token).to_ascii_lowercase();
    let runner = runner.strip_suffix(".exe").unwrap_or(&runner);
    if !RUNNERS.contains(&runner) {
        return None;
    }

    let mut script_token = None;
    for token in tokens.iter().skip(1) {
        if token == "--" || token.starts_with('-') {
            continue;
        }
        script_token = Some(token.as_str());
        break;
    }
    let script_token = script_token?;
    if SCRIPT_EXTENSIONS
        .iter()
        .any(|extension| script_token.to_ascii_lowercase().ends_with(extension))
    {
        return Some(script_token);
    }

    None
}

fn detect_skill_script_run(
    outcome: &impl ImplicitSkillLookup,
    tokens: &[String],
    workdir: &AbsolutePathBuf,
) -> Option<SkillMetadata> {
    let script_token = script_run_token(tokens)?;
    let script_path = Path::new(script_token);
    let script_path = canonicalize_if_exists(&workdir.join(script_path));

    for path in script_path.ancestors() {
        if let Some(candidate) = outcome.implicit_skill_for_scripts_dir(&path) {
            return Some(candidate.clone());
        }
    }

    None
}

fn detect_skill_doc_read(
    outcome: &impl ImplicitSkillLookup,
    tokens: &[String],
    workdir: &AbsolutePathBuf,
) -> Option<SkillMetadata> {
    if let Some(path) = powershell_doc_path(tokens) {
        let candidate_path = canonicalize_if_exists(&workdir.join(Path::new(path)));
        if let Some(candidate) = outcome.implicit_skill_for_doc_path(&candidate_path) {
            return Some(candidate.clone());
        }
    }

    for command in parse_command_impl(tokens) {
        if let ParsedCommand::Read { path, .. } = command {
            let candidate_path = canonicalize_if_exists(&workdir.join(path.as_path()));
            if let Some(candidate) = outcome.implicit_skill_for_doc_path(&candidate_path) {
                return Some(candidate.clone());
            }
        }
    }

    None
}

fn powershell_doc_path(tokens: &[String]) -> Option<&str> {
    let command = command_basename(tokens.first()?).to_ascii_lowercase();
    if !matches!(command.as_str(), "get-content" | "gc" | "type") {
        return None;
    }

    let mut expects_path = false;
    for token in tokens.iter().skip(1) {
        if expects_path {
            return Some(token);
        }
        if matches!(
            token.to_ascii_lowercase().as_str(),
            "-path" | "-literalpath"
        ) {
            expects_path = true;
        } else if !token.starts_with('-') {
            return Some(token);
        }
    }
    None
}

fn powershell_path_to_uri(path: &str, workdir: &PathUri) -> Option<PathUri> {
    let path = path.replace('\\', "/");
    if path.starts_with("//") {
        PathUri::parse(&format!("file:{path}")).ok()
    } else if path.as_bytes().get(1) == Some(&b':') && path.as_bytes().get(2) == Some(&b'/') {
        PathUri::parse(&format!("file:///{path}")).ok()
    } else {
        workdir.join(&path).ok()
    }
}

fn command_basename(command: &str) -> String {
    Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command)
        .to_string()
}

fn canonicalize_if_exists(path: &AbsolutePathBuf) -> AbsolutePathBuf {
    path.canonicalize().unwrap_or_else(|_| path.clone())
}

#[cfg(test)]
#[path = "invocation_tests.rs"]
mod tests;
