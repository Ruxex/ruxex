#![warn(uncommented_anonymous_literal_argument)]

fn describe(prefix: &str, suffix: &str) {
    let _ = (prefix, suffix);
}

fn main() {
    describe("provider", r"https://api.example.com/v1");
}
