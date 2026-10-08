# Remove Terminal Pet Feature Design

## Goal

Remove the terminal Pet feature completely from the Genex repository. After the
change, the TUI must have no Pet picker, ambient Pet rendering, Pet animation or
notification path, Pet asset download/cache path, Pet-specific terminal image
protocol handling, or Pet-specific configuration surface.

## Scope and assumptions

- This is an unreleased repository-owned feature; existing Pet configuration and
  cached assets do not require a compatibility migration.
- The work is performed directly on the current `main` checkout and must not
  overwrite unrelated pre-existing working-tree changes.
- The word `pet` in generic image-generation samples, historical records, or
  third-party/external catalog references is not a Genex runtime feature and is
  retained unless the producer/consumer audit proves otherwise.

## Audited producer/consumer decisions

| Surface | Producers and consumers | Decision |
| --- | --- | --- |
| TUI Pet model, catalog, manifests, animations, frame extraction and image rendering | `crates/tui/src/pets/**`, `chatwidget/pets.rs`, `app/pets.rs`, `tui.rs`, chatwidget/app event consumers | `remove` |
| `/pets` and `/pet` slash command, Picker, preview and selection flow | `slash_command.rs`, `chatwidget/slash_dispatch.rs`, Picker callbacks, `AppEvent` dispatch | `remove` |
| Ambient Pet lifecycle and notification hooks | TUI constructor, redraw loop, tool-request/turn-runtime notification producers, wrap-width calculation | `remove` |
| Pet configuration and public type | `genex_config::TuiConfig.pet`, `pet_anchor`, `TuiPetAnchor`, `genex-core` settings projection, `genex-core-api` re-export | `remove` |
| Pet config mutation and persisted schema | `tui_pet_edit`, config tests, JSON schema, generated schema/precomputed artifacts | `remove` |
| Pet-specific session logging | `crates/tui/src/session_log.rs` Pet event serialization | `remove` |
| Pet CDN, cache and terminal detection dependencies | Pet asset pack and image protocol modules, plus Pet-only TUI state/client plumbing | `remove` |
| Generic or historical uses of the word `pet` | ImageGen sample prompt, dynamic skill test wording, historical/provider catalog documentation | `retain` unless a runtime producer/consumer audit identifies a Pet feature dependency |

## Design

1. Delete the entire `crates/tui/src/pets/` module tree and its test-only helpers.
2. Remove all Pet fields, constants, imports, methods, events, event handlers,
   notification calls, redraw calls, layout reservations, and test cases from
   `crates/tui` and related crates. The normal TUI redraw and transcript layout
   must remain unchanged apart from the absence of Pet-specific branches.
3. Remove `[tui].pet` and `[tui].pet_anchor` from configuration types, the
   `TuiPetAnchor` type, config projections/edit builders, public re-exports, and
   checked-in generated schema artifacts. Existing unknown keys may be rejected
   by the normal `deny_unknown_fields` behavior; no compatibility alias is added.
4. Remove Pet-specific session-log serialization. Other session-log event
   serialization remains intact.
5. Remove only dependencies or imports proven to be Pet-only. Shared image,
   URL, hashing, UUID, terminal-detection, and base64 capabilities must remain
   if other production consumers still use them.
6. Update the pre-release compatibility inventory and candidate queue with the
   audited `remove` decision and evidence. Do not rewrite unrelated migration
   entries.

## Verification

- Use `rg` to confirm no production Pet symbols, config keys, slash commands,
  CDN/cache paths, or Pet event variants remain, excluding explicitly retained
  generic/historical text.
- Run `just generate` and inspect its diff because checked-in schemas and
  generated indexes may change.
- Run `just check --tests` under the repository's user systemd memory scope,
  then run `just check` under the same scope.
- Do not run standalone tests unless explicitly requested.
- Confirm `git diff --check` passes and verify unrelated pre-existing changes are
  still present and untouched.

## Success criteria

- No executable Genex code can select, load, persist, download, cache, animate,
  render, preview, or notify a terminal Pet.
- The configuration schema and public configuration API contain no Pet-specific
  fields or types.
- The TUI compiles with its normal redraw, input, transcript, and notification
  behavior intact.
- Remaining `pet` text is explainable as retained generic, historical, or
  external content rather than an active Pet feature.
