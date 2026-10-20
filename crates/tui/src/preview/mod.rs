use thiserror::Error;

mod design;
mod scenes;
mod svg;

#[derive(Debug, Error)]
pub enum PreviewError {
    #[error("invalid preview design: {0}")]
    InvalidDesign(String),
    #[error("failed to write preview: {0}")]
    Io(#[from] std::io::Error),
}

pub use design::{PreviewBrand, PreviewDesign, PreviewScene, PreviewViewport};

pub fn render_preview(
    design: &PreviewDesign,
    scene: PreviewScene,
    viewport: PreviewViewport,
) -> Result<String, PreviewError> {
    viewport.validate()?;
    let buffer = scenes::render_scene(design, scene, viewport.clone())?;
    svg::buffer_to_svg(&buffer, viewport)
}
