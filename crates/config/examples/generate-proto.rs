use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(proto_dir_arg) = std::env::args().nth(1) else {
        eprintln!("Usage: generate-proto <proto-dir>");
        std::process::exit(1);
    };

    let proto_dir = PathBuf::from(proto_dir_arg);
    let proto_file = proto_dir.join("codex.thread_config.v1.proto");

    let mut config = tonic_prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);

    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir(&proto_dir)
        .compile_with_config(config, &[proto_file], &[proto_dir])?;

    Ok(())
}
