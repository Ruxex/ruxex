use super::parse_stat;
use pretty_assertions::assert_eq;

#[test]
fn parses_start_ticks_after_comm_with_spaces_and_parentheses() {
    let stat =
        b"123 (codex \xff) worker) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 987654321 20";
    assert_eq!(parse_stat(stat).unwrap(), ("S".to_string(), 987654321));
}

#[test]
fn reads_earlier_linux_identity_field() {
    use crate::backend::pid::PidRecord;
    let mut expected = serde_json::json!({
        "pid": 42,
        "processStartTime": "legacy timestamp",
        "linuxProcessIdentity": {"bootId": "boot", "startTicks": 123},
    });
    let record: PidRecord = serde_json::from_value(expected.clone()).unwrap();
    expected["processIdentity"] = expected
        .as_object_mut()
        .unwrap()
        .remove("linuxProcessIdentity")
        .unwrap();
    assert_eq!(serde_json::to_value(record).unwrap(), expected);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[tokio::test]
async fn previous_boot_does_not_require_process_inspection() {
    let (_, identity) = super::read_process_details(std::process::id())
        .await
        .unwrap();
    let mut record = serde_json::to_value(identity).unwrap();
    record["bootId"] = "previous boot".into();
    let identity: super::ProcessIdentity = serde_json::from_value(record).unwrap();
    // An invalid PID would make process inspection fail if it preceded the boot check.
    assert_eq!(
        identity.matches_process(u32::MAX).await.unwrap(),
        (false, false)
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn reused_pid_can_belong_to_root() {
    let (_, identity) = super::read_process_details(std::process::id())
        .await
        .unwrap();
    // launchd is root-owned: PROC_PIDTBSDINFO returns EPERM for an ordinary caller.
    assert_eq!(
        identity.matches_process(/*pid*/ 1).await.unwrap(),
        (false, false)
    );
}
