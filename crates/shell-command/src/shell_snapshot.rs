//! Capture, prepare credential-safe state, and render restorable shell snapshots.

#[path = "shell_snapshot_capture.rs"]
mod capture;
#[path = "shell_snapshot_credentials.rs"]
mod credentials;
#[path = "shell_snapshot_exports.rs"]
mod exports;
#[path = "shell_snapshot_literals.rs"]
mod literals;
#[path = "shell_snapshot_render.rs"]
mod render;

pub use capture::CapturedSnapshot;
pub use capture::CapturedStartupEnvironment;
pub use capture::SnapshotCaptureOptions;
pub use capture::snapshot_capture_script;
pub use credentials::PreparedSnapshot;
pub use credentials::SnapshotCredentialEnvironment;
pub use credentials::prepare_snapshot_credentials;

use capture::BASH_SH_SNAPSHOT_HEADER;

#[cfg(all(test, unix))]
#[path = "shell_snapshot_tests.rs"]
mod tests;

/// Returns the POSIX shell helper used to resolve supported `ENV` startup paths.
///
/// The helper deliberately supports only non-evaluating path forms. Unsupported
/// shell expressions are returned unchanged rather than executed.
pub fn posix_env_path_expansion_function() -> &'static str {
    r#"__ruxex_snapshot_expand_env() (
  set +u
  __ruxex_snapshot_getenv() {
    # Preserve the value separately from lookup status and its formatting newline.
    __ruxex_env_expanded=$(
      if command -v printenv >/dev/null 2>&1; then
        printenv "$1"
      elif [ "$1" = PATH ]; then
        [ "${PATH+x}" = x ] && printf '%s\n' "$PATH"
      else
        command -p printenv "$1"
      fi && printf '.'
    ) || return 1
    __ruxex_env_expanded=${__ruxex_env_expanded%?}
    __ruxex_env_expanded=${__ruxex_env_expanded%?}
  }
  __ruxex_env_file=$1
  case "$__ruxex_env_file" in
    '~/'*) __ruxex_env_file="${HOME-}/${__ruxex_env_file#*/}" ;;
    '${PATH%%:*}') __ruxex_env_file="${PATH%%:*}" ;;
    '${PATH%%:*}/'*) __ruxex_env_file="${PATH%%:*}/${__ruxex_env_file#*/}" ;;
    '${'*)
      __ruxex_env_body=${__ruxex_env_file#\$\{}
      case "$__ruxex_env_body" in
        *\}*)
          __ruxex_env_name=${__ruxex_env_body%%\}*}
          __ruxex_env_suffix=${__ruxex_env_body#*\}}
          case "$__ruxex_env_name" in
            *:-*)
              __ruxex_env_default=${__ruxex_env_name#*:-}
              __ruxex_env_name=${__ruxex_env_name%%:-*}
              __ruxex_env_has_default=1
              ;;
            *) __ruxex_env_has_default= ;;
          esac
          case "$__ruxex_env_name" in
            ''|[0-9]*|*[!A-Za-z0-9_]*) ;;
            *)
              case "$__ruxex_env_suffix" in
                ''|/*)
                  if __ruxex_snapshot_getenv "$__ruxex_env_name" 2>/dev/null &&
                    { [ -n "$__ruxex_env_expanded" ] || [ -z "$__ruxex_env_has_default" ]; }; then
                    __ruxex_env_file="$__ruxex_env_expanded$__ruxex_env_suffix"
                  elif [ -n "$__ruxex_env_has_default" ]; then
                    __ruxex_env_default=$(__ruxex_snapshot_expand_env "$__ruxex_env_default")
                    __ruxex_env_file="$__ruxex_env_default$__ruxex_env_suffix"
                  fi
                  ;;
              esac
              ;;
          esac
          ;;
      esac
      ;;
    '$'*)
      __ruxex_env_name=${__ruxex_env_file%%/*}
      __ruxex_env_name=${__ruxex_env_name#\$}
      case "$__ruxex_env_name" in
        ''|[0-9]*|*[!A-Za-z0-9_]*) ;;
        *)
          if __ruxex_snapshot_getenv "$__ruxex_env_name" 2>/dev/null; then
            if [ "$__ruxex_env_file" = "\$$__ruxex_env_name" ]; then
              __ruxex_env_file=$__ruxex_env_expanded
            else
              __ruxex_env_file="$__ruxex_env_expanded/${__ruxex_env_file#*/}"
            fi
          fi
          ;;
      esac
      ;;
  esac
  printf '%s' "$__ruxex_env_file"
)"#
}

#[derive(Clone, Copy)]
pub enum SnapshotStartup {
    Interactive,
    NonInteractive,
}
