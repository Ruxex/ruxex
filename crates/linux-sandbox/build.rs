fn main() {
    println!("cargo:rerun-if-env-changed=GENEX_BWRAP_SHA256");
}
