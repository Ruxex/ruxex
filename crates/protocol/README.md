# ruxex-protocol

This crate defines the protocol types used by Ruxex hosts, including both
internal types for communication between `ruxex-core` and `ruxex-tui`, and
external types used with `ruxex-app-server`.

This crate should have minimal dependencies.

Ideally, we should avoid "material business logic" in this crate, as we can always introduce `Ext`-style traits to add functionality to types in other crates.
