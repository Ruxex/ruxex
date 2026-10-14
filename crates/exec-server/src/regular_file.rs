use std::io;
use std::path::Path;
use tokio::io::AsyncReadExt;

pub(crate) async fn open(path: &Path) -> io::Result<tokio::fs::File> {
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true);
    configure_open(&mut options);

    let file = options.open(path).await?;
    if !is_disk_file(&file) || !file.metadata().await?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path `{}` is not a file", path.display()),
        ));
    }
    Ok(file)
}

/// Reads a regular UTF-8 file without following a symlink at its final path component.
pub async fn read_sensitive_file_to_string(path: &Path) -> io::Result<String> {
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true);
    configure_open(&mut options);

    #[cfg(unix)]
    options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);

    let mut file = options.open(path).await?;
    let metadata = file.metadata().await?;
    if !is_disk_file(&file) || !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path `{}` is not a regular file", path.display()),
        ));
    }

    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    Ok(contents)
}

#[cfg(unix)]
fn configure_open(options: &mut tokio::fs::OpenOptions) {
    options.custom_flags(libc::O_NONBLOCK);
}

fn is_disk_file(_file: &tokio::fs::File) -> bool {
    true
}

#[cfg(test)]
#[path = "regular_file_tests.rs"]
mod tests;
