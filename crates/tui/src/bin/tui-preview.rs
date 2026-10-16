use clap::Parser;
use ruxex_tui::preview::{PreviewDesign, PreviewScene, render_preview};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "ruxex-tui-preview", about = "Render a Ruxex TUI design to SVG")]
struct Args {
    /// TOML design file to render.
    #[arg(long)]
    design: PathBuf,
    /// Scene to render: welcome, composer, conversation, status, or all.
    #[arg(long, default_value = "all")]
    scene: String,
    /// SVG output path.
    #[arg(long)]
    output: PathBuf,
    /// Optional viewport width override.
    #[arg(long)]
    width: Option<u16>,
    /// Optional viewport height override.
    #[arg(long)]
    height: Option<u16>,
}

fn main() {
    if let Err(error) = run(Args::parse()) {
        eprintln!("ruxex-tui-preview: {error}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let input = fs::read_to_string(&args.design)
        .map_err(|error| format!("could not read {}: {error}", args.design.display()))?;
    let design = PreviewDesign::from_toml(&input)?;
    let scene = args.scene.parse::<PreviewScene>()?;
    let mut viewport = design.viewport.clone();
    if let Some(width) = args.width {
        viewport.width = width;
    }
    if let Some(height) = args.height {
        viewport.height = height;
    }
    let svg = render_preview(&design, scene, viewport)?;
    write_atomic(&args.output, svg.as_bytes())?;
    Ok(())
}

fn write_atomic(path: &PathBuf, contents: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
