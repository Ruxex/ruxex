# `codex doctor`

## Entry

`DoctorCommand` is dispatched at [`main.rs#L1428`](../../../../crates/cli/src/main.rs#L1428) into [`doctor::run_doctor`](../../../../crates/cli/src/doctor.rs#L308).

## Scope

Doctor is read-mostly. It inspects installation/package layout, config, auth, terminal/runtime, disk/state paths, Git, network/provider reachability, MCP, sandbox/security, and thread inventory. It produces redacted serializable check rows that can back human and JSON reports.

```text
codex doctor
  -> load local config and installation context
  -> run bounded sync/async checks
  -> redact and aggregate check rows
  -> render human report or JSON
```

Checks should diagnose and provide remediation without repairing state or starting long-lived services. Network checks are bounded probes; they do not prove that a complete agent turn will succeed.

See [configuration](../common/configuration.md), [authentication](../common/authentication.md), [transport/server](../common/transport-and-server.md), and [output/errors](../common/output-and-errors.md).
