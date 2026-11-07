use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

pub const MAX_DISCS: usize = 15;
const DESCRIPTOR_BYTES: u64 = 64 * 1024;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn text(path: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(DESCRIPTOR_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > DESCRIPTOR_BYTES {
        return Err(invalid("disc descriptor exceeds 64 KiB"));
    }
    String::from_utf8(bytes).map_err(|_| invalid("disc descriptor is not UTF-8"))
}

fn inside(root: &Path, path: &Path) -> io::Result<PathBuf> {
    let resolved = path.canonicalize()?;
    let metadata = std::fs::metadata(&resolved)?;
    if !resolved.starts_with(root) || !metadata.is_file() || metadata.len() > u32::MAX as u64 {
        return Err(invalid(
            "disc file is outside Games/PS or exceeds FAT32 limits",
        ));
    }
    // Match the core: extension and relative references belong to the supplied
    // filename, not a symlink target that may have a different name or parent.
    Ok(path.to_path_buf())
}

fn sibling(root: &Path, descriptor: &Path, name: &str) -> io::Result<PathBuf> {
    let relative = Path::new(name);
    if name.is_empty()
        || name.contains('\\')
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(invalid(
            "disc references must be relative paths inside Games/PS",
        ));
    }
    inside(root, &descriptor.parent().unwrap_or(root).join(relative))
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn validate_disc(root: &Path, path: &Path) -> io::Result<()> {
    match extension(path).as_str() {
        "chd" | "pbp" => Ok(()),
        "cue" => {
            let mut tracks = 0;
            for line in text(path)?.lines() {
                if line.len() >= 255 || line.contains(['\0', '\t']) {
                    return Err(invalid("CUE lines must fit the core's parser"));
                }
                let mut words = line.trim().splitn(2, char::is_whitespace);
                if !words.next().is_some_and(|w| w.eq_ignore_ascii_case("FILE")) {
                    continue;
                }
                let rest = words.next().unwrap_or_default().trim();
                let name = if let Some(quoted) = rest.strip_prefix('"') {
                    quoted.split_once('"').map(|(name, _)| name)
                } else {
                    rest.split_whitespace().next()
                }
                .ok_or_else(|| invalid("invalid CUE FILE entry"))?;
                if Path::new(name).components().count() != 1 {
                    return Err(invalid("track files must be beside their CUE descriptor"));
                }
                sibling(root, path, name)?;
                tracks += 1;
                if tracks > 99 {
                    return Err(invalid("CUE has more than 99 track files"));
                }
            }
            if tracks == 0 {
                return Err(invalid("CUE has no track files"));
            }
            Ok(())
        }
        _ => Err(invalid("PS1 content must be CUE, CHD, PBP or M3U")),
    }
}

/// Resolve one logical disc set without extracting, moving or modifying its files.
pub fn contents(root: &Path, path: &Path) -> io::Result<Vec<PathBuf>> {
    let root = root.canonicalize()?;
    let path = inside(&root, path)?;
    if extension(&path) != "m3u" {
        validate_disc(&root, &path)?;
        return Ok(vec![path]);
    }
    let mut discs = Vec::new();
    for line in text(&path)?.lines() {
        if line.len() >= 1023 || line.contains('\0') || line.trim() != line {
            return Err(invalid(
                "M3U entries must be unpadded paths under 1023 bytes",
            ));
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if discs.len() == MAX_DISCS {
            return Err(invalid("M3U has more than 15 discs"));
        }
        let disc = sibling(&root, &path, line)?;
        validate_disc(&root, &disc)?;
        discs.push(disc);
    }
    if discs.is_empty() {
        return Err(invalid("M3U has no discs"));
    }
    Ok(discs)
}
