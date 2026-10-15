// Referencing the Cargo-provided path makes `cargo nextest archive` build and
// include the host binary alongside this package's test archive.
const PROVIDER_HOST_BINARY: &str = env!("CARGO_BIN_EXE_ruxex-provider-host-example");

#[test]
fn provider_host_binary_is_built_for_test_archives() {
    assert!(!PROVIDER_HOST_BINARY.is_empty());
}
