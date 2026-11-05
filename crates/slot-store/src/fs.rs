//! Optional process-wide write boundary for the stock SD runtime.
pub use std::fs::*;

use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

static ROOT: OnceLock<WriteRoot> = OnceLock::new();

struct WriteRoot {
    path: PathBuf,
    identity: (u64, u64),
}

fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "SD write boundary refused path or changed mount",
    )
}

#[cfg(unix)]
fn identity(meta: &Metadata) -> io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Ok((meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn identity(_: &Metadata) -> io::Result<(u64, u64)> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "SD confinement requires Unix",
    ))
}

impl WriteRoot {
    fn check(&self, path: &Path) -> io::Result<()> {
        if self.path.canonicalize()? != self.path
            || identity(&std::fs::metadata(&self.path)?)? != self.identity
            || !path.is_absolute()
            || !path.starts_with(&self.path)
            || path.components().any(|c| matches!(c, Component::ParentDir))
        {
            return Err(refused());
        }
        for ancestor in path.ancestors() {
            if !ancestor.starts_with(&self.path) {
                break;
            }
            match std::fs::symlink_metadata(ancestor) {
                Ok(meta) => {
                    if meta.file_type().is_symlink()
                        || (!meta.is_file() && !meta.is_dir())
                        || identity(&meta)?.0 != self.identity.0
                    {
                        return Err(refused());
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if meta.is_file() && meta.nlink() != 1 {
                            return Err(refused());
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn tree(&self, path: &Path) -> io::Result<()> {
        self.check(path)?;
        if std::fs::symlink_metadata(path)?.is_dir() {
            for entry in std::fs::read_dir(path)? {
                self.tree(&entry?.path())?;
            }
        }
        Ok(())
    }
}

/// Install only after the caller verifies that this root is on the intended SD mount.
/// Reject the entire existing tree before enabling any writes or worker threads.
pub fn confine_writes(root: &Path) -> io::Result<()> {
    if root.canonicalize()? != root || !root.is_dir() {
        return Err(refused());
    }
    let boundary = WriteRoot {
        path: root.to_path_buf(),
        identity: identity(&std::fs::metadata(root)?)?,
    };
    boundary.tree(root)?;
    ROOT.set(boundary).map_err(|_| refused())
}

pub fn check_write(path: impl AsRef<Path>) -> io::Result<()> {
    if let Some(root) = ROOT.get() {
        root.check(path.as_ref())?;
    }
    Ok(())
}

pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    check_write(&path)?;
    std::fs::create_dir_all(path)
}

pub fn create_file(path: impl AsRef<Path>) -> io::Result<File> {
    check_write(&path)?;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(target_os = "linux")]
    if ROOT.get().is_some() {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

pub fn append_file(path: impl AsRef<Path>) -> io::Result<File> {
    check_write(&path)?;
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(target_os = "linux")]
    if ROOT.get().is_some() {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

pub fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> io::Result<()> {
    use std::io::Write;
    create_file(path)?.write_all(bytes.as_ref())
}

pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    check_write(&from)?;
    check_write(&to)?;
    std::fs::rename(from, to)
}

pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    check_write(&path)?;
    std::fs::remove_file(path)
}

pub fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    if let Some(root) = ROOT.get() {
        if path.as_ref() == root.path {
            return Err(refused());
        }
        root.tree(path.as_ref())?;
    }
    std::fs::remove_dir_all(path)
}
