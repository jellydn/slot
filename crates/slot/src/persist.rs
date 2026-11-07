use std::io::Read;
use std::path::{Path, PathBuf};

use slot_store::{atomic_write, write_slot_state, Core, Platform, SlotState, StateRing};

pub trait Snapshot {
    fn state(&self) -> Option<Vec<u8>>;
    fn save_ram(&self) -> Option<Vec<u8>>;
    fn thumb(&self) -> Option<Vec<u8>>;
    fn load(&self, state: Vec<u8>);

    fn reset(&self) {}

    fn supports_states(&self) -> bool {
        true
    }

    fn resume_trusted(&self) -> bool {
        true
    }

    fn save_ram_trusted(&self) -> bool {
        true
    }
}

pub fn flush(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
    state: Option<&[u8]>,
    sav: Option<&[u8]>,
) -> std::io::Result<()> {
    if let Some(state) = state {
        let ring = StateRing::new(root, platform, core, stem);
        if !matches!(ring.read_resume(), Ok(Some(old)) if old == state) {
            ring.write_resume(state)?;
        }
    }
    if let Some(sav) = sav {
        write_sav_for(root, platform, core, stem, sav)?;
    }
    Ok(())
}

pub fn eject(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
    state: Option<&[u8]>,
    sav: Option<&[u8]>,
    slot: &SlotState,
) -> std::io::Result<()> {
    flush(root, platform, core, stem, state, sav)?;
    let mut slot = slot.clone();
    slot.cart = None;
    slot.cart_platform = None;
    write_slot_state(root, &slot)
}

pub fn write_sav(root: &Path, platform: Platform, stem: &str, sav: &[u8]) -> std::io::Result<bool> {
    write_sav_for(root, platform, default_core(platform), stem, sav)
}

pub fn write_sav_for(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
    sav: &[u8],
) -> std::io::Result<bool> {
    let path = sav_path_for(root, platform, core, stem);
    if core == Core::SwanStation {
        swan_card(sav)?;
    }
    let old = read_sav_for(root, platform, core, stem);
    if core == Core::SwanStation && old.is_err() {
        return old.map(|_| false);
    }
    if let Ok(Some(old)) = old {
        if old == sav {
            return Ok(false);
        }
    }
    if let Some(dir) = path.parent() {
        slot_store::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, sav)?;
    Ok(true)
}

pub fn read_sav(root: &Path, platform: Platform, stem: &str) -> std::io::Result<Option<Vec<u8>>> {
    read_sav_for(root, platform, default_core(platform), stem)
}

pub fn read_sav_for(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
) -> std::io::Result<Option<Vec<u8>>> {
    if platform.native() {
        return match std::fs::read(sav_path_for(root, platform, core, stem)) {
            Ok(bytes) => {
                if core == Core::SwanStation {
                    swan_card(&bytes)?;
                }
                Ok(Some(bytes))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        };
    }
    let srm = crate::root::saves_dir(root)
        .join(platform.dir_name())
        .join(format!("{stem}.srm"));
    for path in [sav_path_for(root, platform, core, stem), srm] {
        match std::fs::read(&path) {
            Ok(bytes) if bytes.starts_with(RZIP_MAGIC) => return unrzip(&bytes).map(Some),
            Ok(bytes) => return Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(None)
}

fn swan_card(bytes: &[u8]) -> std::io::Result<()> {
    if bytes.len() != 128 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "SwanStation card must be exactly 128 KiB; preserve the existing file",
        ));
    }
    Ok(())
}

const RZIP_MAGIC: &[u8] = b"#RZIPv\x01#";

fn unrzip(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "bad rzip save");
    let total = bytes.get(12..20).ok_or_else(bad)?;
    let total = u64::from_le_bytes(total.try_into().map_err(|_| bad())?);
    let mut rest = &bytes[20..];
    let mut out = Vec::new();
    while let Some((len, body)) = rest.split_first_chunk::<4>() {
        let len = u32::from_le_bytes(*len) as usize;
        let chunk = body.get(..len).ok_or_else(bad)?;
        flate2::read::ZlibDecoder::new(chunk).read_to_end(&mut out)?;
        rest = &body[len..];
    }
    if out.len() as u64 == total {
        Ok(out)
    } else {
        Err(bad())
    }
}

pub fn read_resume(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
) -> std::io::Result<Option<Vec<u8>>> {
    StateRing::new(root, platform, core, stem).read_resume()
}

pub fn core_save_dir(root: &Path, platform: Platform, stem: &str) -> PathBuf {
    core_save_dir_for(root, platform, default_core(platform), stem)
}

pub fn core_save_dir_for(root: &Path, platform: Platform, core: Core, stem: &str) -> PathBuf {
    let saves = crate::root::saves_dir(root);
    if !platform.native() {
        return saves;
    }
    use sha2::{Digest, Sha256};
    let identity = format!("{:x}", Sha256::digest(stem.as_bytes()));
    saves
        .join(platform.dir_name())
        .join(core.as_str())
        .join(identity)
}

fn default_core(platform: Platform) -> Core {
    match platform {
        Platform::Ps => Core::PcsxRearmed,
        Platform::Pico8 => Core::Fake08,
        _ => Core::Mgba,
    }
}

fn sav_path_for(root: &Path, platform: Platform, core: Core, stem: &str) -> PathBuf {
    if platform.native() {
        return core_save_dir_for(root, platform, core, stem).join("memory.sav");
    }
    crate::root::saves_dir(root)
        .join(platform.dir_name())
        .join(format!("{stem}.sav"))
}
