//! Permission-profile path grammar. Path facts and resolution are owned by
//! the shared configuration context; symbolic paths preserve native spelling.

use std::io;

use genex_config::ConfigPathContext;
use genex_utils_path_uri::LegacyAppPathString;
use genex_utils_path_uri::PathUri;

pub(super) fn contains_glob(path: &str, _context: &ConfigPathContext) -> io::Result<bool> {
    Ok(contains_glob_chars(path))
}

pub(super) fn absolute_path(path: &str, context: &ConfigPathContext) -> io::Result<PathUri> {
    let convention = context.convention();
    if convention.home_relative_suffix(path).is_none()
        && LegacyAppPathString::from_string(path)
            .to_path_uri(convention)
            .is_err()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("filesystem path `{path}` must be absolute, use `~/...`, or start with `:`"),
        ));
    }
    context
        .resolve_path(path)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
}

pub(super) fn relative_subpath(subpath: &str, context: &ConfigPathContext) -> io::Result<String> {
    let convention = context.convention();
    let mut components = convention.path_segments(subpath);
    let first = components.next();
    if !subpath.is_empty()
        && !matches!(first, Some("" | "." | ".."))
        && !components.any(|component| component == "..")
    {
        // Preserve spelling after the same validation as native components:
        // interior `.` and repeated separators stay in symbolic rules.
        return Ok(subpath.to_string());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "filesystem subpath `{subpath}` must be a descendant path without `.` or `..` components"
        ),
    ))
}

fn contains_glob_chars(path: &str) -> bool {
    path.chars().any(|ch| matches!(ch, '*' | '?' | '[' | ']'))
}

#[cfg(test)]
#[path = "permission_path_tests.rs"]
mod tests;
