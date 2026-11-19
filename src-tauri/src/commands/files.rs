//! Saving an exported image to a path the person chose in a save dialog.
//!
//! The webview can't write files, and a download link does nothing in a
//! desktop webview, so the diagram's SVG and PNG come here.

use crate::errors::AppError;
use base64::Engine as _;
use std::path::{Path, PathBuf};

/// Larger than any diagram should be; a bound on what the webview can ask
/// the app to write.
const MAX_BYTES: usize = 64 * 1024 * 1024;

fn destination(path: &str, extension: &str) -> Result<PathBuf, AppError> {
    let invalid = || AppError::new("validation", "Choose where to save the file again.");
    let mut path = PathBuf::from(path);
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(invalid());
    }
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(invalid());
    }
    path.set_extension(extension);
    Ok(path)
}

/// Writes a diagram as SVG text or PNG bytes (base64, as a canvas gives it).
#[tauri::command]
pub async fn save_image(
    path: String,
    svg: Option<String>,
    png_base64: Option<String>,
) -> Result<String, AppError> {
    let (path, bytes) = match (svg, png_base64) {
        (Some(svg), None) => (destination(&path, "svg")?, svg.into_bytes()),
        (None, Some(png)) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(png.trim())
                .map_err(|_| AppError::new("validation", "The image couldn't be read."))?;
            // A PNG starts with its signature; anything else isn't one.
            if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err(AppError::new("validation", "The image couldn't be read."));
            }
            (destination(&path, "png")?, bytes)
        }
        _ => return Err(AppError::new("validation", "Nothing to save.")),
    };
    if bytes.len() > MAX_BYTES {
        return Err(AppError::new(
            "validation",
            "The image is too large to save.",
        ));
    }
    let shown = path.to_string_lossy().into_owned();
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&path, bytes))
        .await
        .map_err(|_| AppError::background_task())?
        .map_err(|_| AppError::new("io", "The file couldn't be saved there."))?;
    Ok(shown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_only_where_asked_and_with_the_right_extension() {
        assert!(destination("relative.png", "png").is_err());
        assert!(destination("/no/such/folder/x.png", "png").is_err());
        let dir = std::env::temp_dir();
        let path = destination(dir.join("diagram.txt").to_str().unwrap(), "svg").unwrap();
        assert_eq!(path.extension().unwrap(), "svg");
    }
}
