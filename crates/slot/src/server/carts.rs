use slot_store::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

use image_webp::{ColorType, WebPEncoder};
use slot_store::{Cart, Platform};
use slot_ui::{cart_backing, cart_face_with, cart_interior, gb_shell_of, shows_interior, CartFace};

use super::games::{modified_ms, IMMUTABLE};
use super::{Reply, Request};
use crate::label_cache;

const MAX_RENDERS: usize = 2;
const RENDER_VERSION: u32 = 1;
const NATIVE_RENDER_VERSION: u32 = 2;
const STOP_POLL: Duration = Duration::from_millis(50);
const HASH_LEN: usize = 16;

static RENDERING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static FINISHED: Condvar = Condvar::new();

fn rendering() -> MutexGuard<'static, Vec<PathBuf>> {
    RENDERING.lock().unwrap_or_else(|e| e.into_inner())
}

struct Turn<'a>(&'a Path);

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        rendering().retain(|p| p != self.0);
        FINISHED.notify_all();
    }
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn stem_prefix(stem: &str) -> String {
    format!("{:016x}-", fnv(stem.as_bytes()))
}

pub(super) fn key(cart: &Cart) -> String {
    let mtime = cart.label.as_deref().map_or(0, modified_ms);
    let shell = cart
        .shell
        .map_or_else(|| "auto".to_string(), |c| c.to_value());
    let version = if cart.platform.native() {
        NATIVE_RENDER_VERSION
    } else {
        RENDER_VERSION
    };
    let state = format!("{version}\0{}\0{mtime}\0{shell}", cart.stem);
    format!("{}{:016x}", stem_prefix(&cart.stem), fnv(state.as_bytes()))
}

fn folder(root: &Path, platform: Platform) -> PathBuf {
    root.join("Cache")
        .join("server-carts")
        .join(platform.dir_name())
}

fn hash_of(text: &str) -> Option<u64> {
    let plain = text.len() == HASH_LEN
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if plain {
        u64::from_str_radix(text, 16).ok()
    } else {
        None
    }
}

fn file_for(root: &Path, platform: Platform, v: &str) -> Option<PathBuf> {
    let (stem, state) = v.split_once('-')?;
    let (stem, state) = (hash_of(stem)?, hash_of(state)?);
    Some(folder(root, platform).join(format!("{stem:016x}-{state:016x}.webp")))
}

pub(super) fn cached(root: &Path, platform: Platform, req: &Request) -> Option<Reply> {
    if req.method != "GET" && req.method != "HEAD" {
        return None;
    }
    let path = file_for(root, platform, req.query.get("v")?)?;
    path.is_file()
        .then(|| Reply::file(path).with_header("Cache-Control", IMMUTABLE))
}

fn over(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        let sa = u32::from(s[3]);
        let da = u32::from(d[3]) * (255 - sa) / 255;
        let a = sa + da;
        if a == 0 {
            continue;
        }
        for c in 0..3 {
            d[c] = ((u32::from(s[c]) * sa + u32::from(d[c]) * da + a / 2) / a) as u8;
        }
        d[3] = a as u8;
    }
}

fn draw(root: &Path, cart: &Cart) -> CartFace {
    if cart.platform.native() {
        return native_face(cart);
    }
    let mut cart = cart.clone();
    cart.read_header();
    let face = cart_face_with(&cart, label_cache::label_art(root, &cart));
    if !shows_interior(&cart) {
        return face;
    }
    let mut out = cart_interior(&cart);
    over(&mut out.rgba, &cart_backing(gb_shell_of(&cart)).rgba);
    over(&mut out.rgba, &face.rgba);
    out
}

// Web-only sleeves: the native shelf keeps its existing cartridge rendering.
fn native_face(cart: &Cart) -> CartFace {
    let w = 320;
    let h = if cart.platform == Platform::Pico8 {
        410
    } else {
        320
    };
    let mut face = CartFace {
        rgba: vec![0; (w * h * 4) as usize],
        w,
        h,
    };
    for y in 0..h {
        for x in 0..w {
            let colour = if x < 4 || x >= w - 4 || y < 4 || y >= h - 4 {
                [105, 111, 120, 255]
            } else {
                [39, 43, 50, 255]
            };
            let at = ((y * w + x) * 4) as usize;
            face.rgba[at..at + 4].copy_from_slice(&colour);
        }
    }
    sleeve_text(&mut face, cart.platform.name(), 8, 28);
    sleeve_text(&mut face, &slot_ui::label_text(cart), h - 56, 44);

    // Decode the source, not the label cache: that cache crops to the GBA well.
    if let Some((art, sw, sh)) = cart.label.as_deref().and_then(slot_ui::decode_png) {
        if sw > 0 && sh > 0 {
            let scale = ((w - 24) as f32 / sw as f32).min((h - 112) as f32 / sh as f32);
            let aw = (sw as f32 * scale).round().max(1.0) as u32;
            let ah = (sh as f32 * scale).round().max(1.0) as u32;
            let left = (w - aw) / 2;
            let top = 44 + (h - 112 - ah) / 2;
            for y in 0..ah {
                for x in 0..aw {
                    let src = (((y * sh / ah) * sw + x * sw / aw) * 4) as usize;
                    let dst = (((top + y) * w + left + x) * 4) as usize;
                    over(&mut face.rgba[dst..dst + 4], &art[src..src + 4]);
                }
            }
        }
    }
    face
}

fn sleeve_text(face: &mut CartFace, label: &str, top: u32, h: u32) {
    let Some(font) = slot_ui::text::label_font() else {
        return;
    };
    let w = face.w - 24;
    let layout = slot_ui::text::fit_box(font, label, w as f32 - 8.0, h as f32, 2, 18.0, 8.0);
    let mut rgba = vec![0; (w * h * 4) as usize];
    slot_ui::text::draw_centred(&mut rgba, w, h, &layout, [229, 232, 237]);
    for y in 0..h {
        let src = (y * w * 4) as usize;
        let dst = (((top + y) * face.w + 12) * 4) as usize;
        over(
            &mut face.rgba[dst..dst + (w * 4) as usize],
            &rgba[src..src + (w * 4) as usize],
        );
    }
}

fn encode(face: &CartFace) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    WebPEncoder::new(&mut out)
        .encode(&face.rgba, face.w, face.h, ColorType::Rgba8)
        .ok()?;
    Some(out)
}

fn remove_matching(dir: &Path, prefix: &str, keep: Option<&Path>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let keep = keep.and_then(Path::file_name);
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        if Some(name.as_os_str()) != keep && name.to_string_lossy().starts_with(prefix) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

pub(super) fn forget(root: &Path, platform: Platform, stem: &str) {
    remove_matching(&folder(root, platform), &stem_prefix(stem), None);
}

fn store(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = dest.with_extension("tmp");
    let written = fs::create_file(&tmp).and_then(|mut f| {
        f.write_all(bytes)?;
        f.sync_data()
    });
    let moved = written.and_then(|_| fs::rename(&tmp, dest));
    if moved.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    moved
}

pub fn render_once(
    dest: &Path,
    stop: &AtomicBool,
    render: impl FnOnce() -> Option<Vec<u8>>,
) -> bool {
    let mut busy = rendering();
    loop {
        if dest.is_file() {
            return true;
        }
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        if busy.len() < MAX_RENDERS && !busy.iter().any(|p| p == dest) {
            break;
        }
        busy = FINISHED
            .wait_timeout(busy, STOP_POLL)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
    busy.push(dest.to_path_buf());
    drop(busy);
    let _turn = Turn(dest);
    let Some(bytes) = render() else { return false };
    store(dest, &bytes).is_ok()
}

pub fn cart(root: &Path, cart: &Cart, req: &Request) -> Reply {
    if req.method != "GET" && req.method != "HEAD" {
        return Reply::error(405, "method not allowed");
    }
    let key = key(cart);
    let Some(dest) = file_for(root, cart.platform, &key) else {
        return Reply::error(500, "cart render failed");
    };
    let stored = render_once(&dest, &req.stop, || {
        let bytes = encode(&draw(root, cart))?;
        if let Some(dir) = dest.parent() {
            remove_matching(dir, &stem_prefix(&cart.stem), Some(&dest));
        }
        Some(bytes)
    });
    if !stored {
        return Reply::error(503, "cart not rendered");
    }
    let reply = Reply::file(dest);
    if req.query.get("v") == Some(&key) {
        reply.with_header("Cache-Control", IMMUTABLE)
    } else {
        reply
    }
}
