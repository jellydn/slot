use std::path::Path;
use std::sync::OnceLock;

use slot_gfx::{Canvas, Draw, TexId, OUT_H, OUT_W};

use crate::art;

const DEFAULT_PNG: &[u8] = include_bytes!("../../../assets/background_default.png");
const BEZEL_GB_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb.png");
const BEZEL_GBC_PNG: &[u8] = include_bytes!("../../../assets/bezel_gbc.png");
const BEZEL_GB_FIT_3X2_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb_fit_3x2.png");
const BEZEL_GBC_FIT_3X2_PNG: &[u8] = include_bytes!("../../../assets/bezel_gbc_fit_3x2.png");
const BEZEL_GB_INTEGER_4X3_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb_integer_4x3.png");
const BEZEL_GBC_INTEGER_4X3_PNG: &[u8] =
    include_bytes!("../../../assets/bezel_gbc_integer_4x3.png");
const BEZEL_GB_FIT_4X3_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb_fit_4x3.png");
const BEZEL_GBC_FIT_4X3_PNG: &[u8] = include_bytes!("../../../assets/bezel_gbc_fit_4x3.png");
const BLUR_PNG: &[u8] = include_bytes!("../../../assets/background_default_blur.png");

const SCRIM: f32 = 0.62;

pub fn wallpaper_face(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    art::cover(path, w, h)
}

pub fn bezel_face(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    art::cover(path, w, h)
}

pub fn game_bezel_face(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    let (src, sw, sh) = art::decode(path)?;
    panel_bezel(&src, (sw, sh), (w, h), false)
}

/// Original geometric side panels, without third-party art or marks.
/// The compositor always draws the uncropped native picture above this background.
pub fn native_bezel_face(platform: slot_store::Platform, w: u32, h: u32) -> Vec<u8> {
    let pico = platform == slot_store::Platform::Pico8;
    let picture_w = if pico { h } else { h * 4 / 3 };
    let side = w.saturating_sub(picture_w) / 2 * 720 / h;
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        for x in 0..w {
            // Work in height-relative units so bevels and cells keep their shape.
            let edge = x.min(w - 1 - x) * 720 / h;
            let row = y * 720 / h;
            let grain = ((x / 3 + y / 3) % 3) as u8;
            let mut colour = [24 + grain, 27 + grain, 32 + grain];
            if edge < side {
                let shade = (edge * 10 / side) as u8;
                colour = [30 + shade + grain, 34 + shade + grain, 40 + shade + grain];
                if edge < 6 || edge + 6 >= side {
                    colour = [15, 18, 23];
                } else if edge == 18 || row == 32 || row == 687 {
                    colour = [64, 72, 82];
                }
                if pico {
                    if edge > 30 && edge + 30 < side && (64..656).contains(&row) {
                        if edge.is_multiple_of(16) && row.is_multiple_of(16) {
                            colour = [61, 75, 82];
                        }
                        let cell = edge / 16;
                        let band = row / 16;
                        if (band == 7 && cell >= 4 && cell + 5 <= side / 16)
                            || (band == 8 && cell == 4)
                            || (band == 9 && cell == 5)
                        {
                            colour = [85, 145, 148];
                        }
                        if band == 36 && (cell == 4 || cell == 6 || cell == 8) {
                            colour = [181, 126, 96];
                        }
                    }
                    if edge + 15 == side && (64..656).contains(&row) {
                        colour = [85, 145, 148];
                    }
                } else {
                    if edge > 36 && edge + 36 < side && (216..504).contains(&row) && row % 24 < 3 {
                        colour = [19, 23, 29];
                    }
                    let dx = edge as i32 - side as i32 / 2;
                    let dy = row as i32 - 120;
                    let radius = dx * dx + dy * dy;
                    if (529..625).contains(&radius) || (100..144).contains(&radius) {
                        colour = [84, 94, 108];
                    }
                    if row == 600 && edge > 36 && edge + 36 < side {
                        colour = [107, 120, 137];
                    }
                }
            }
            rgba.extend_from_slice(&[colour[0], colour[1], colour[2], 255]);
        }
    }
    rgba
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BezelArt {
    Default,
    Fit3x2,
    Integer4x3,
    Fit4x3,
}

pub fn default_game_bezel_face(colour: bool, kind: BezelArt, w: u32, h: u32) -> Option<Vec<u8>> {
    let png = match (kind, colour) {
        (BezelArt::Default, false) => BEZEL_GB_PNG,
        (BezelArt::Default, true) => BEZEL_GBC_PNG,
        (BezelArt::Fit3x2, false) => BEZEL_GB_FIT_3X2_PNG,
        (BezelArt::Fit3x2, true) => BEZEL_GBC_FIT_3X2_PNG,
        (BezelArt::Integer4x3, false) => BEZEL_GB_INTEGER_4X3_PNG,
        (BezelArt::Integer4x3, true) => BEZEL_GBC_INTEGER_4X3_PNG,
        (BezelArt::Fit4x3, false) => BEZEL_GB_FIT_4X3_PNG,
        (BezelArt::Fit4x3, true) => BEZEL_GBC_FIT_4X3_PNG,
    };
    let (src, sw, sh) = art::decode_png_bytes(png)?;
    match kind {
        BezelArt::Default => panel_bezel(&src, (sw, sh), (w, h), true),
        _ => art::cover_rgba(&src, sw, sh, w, h),
    }
}

const BEZEL_GRAIN_ROWS: usize = 10;

fn panel_bezel(
    src: &[u8],
    (sw, sh): (u32, u32),
    (w, h): (u32, u32),
    extend: bool,
) -> Option<Vec<u8>> {
    if !slot_gfx::framed((w, h)) || sw * 3 == sh * 4 {
        return art::cover_rgba(src, sw, sh, w, h);
    }
    let game_h = slot_gfx::screen_rect((w, h)).3 as usize;
    let inner = art::cover_rgba(src, sw, sh, w, game_h as u32)?;
    let (h, row) = (h as usize, w as usize * 4);
    let gap = h - game_h;
    let top = gap - gap / 2;
    let mut out = vec![0u8; row * h];
    out[top * row..(top + game_h) * row].copy_from_slice(&inner);
    if !extend {
        return Some(out);
    }
    let ping = |d: usize| {
        let p = d % (2 * BEZEL_GRAIN_ROWS);
        p.min(2 * BEZEL_GRAIN_ROWS - 1 - p)
    };
    for d in 0..top {
        let from = ping(d) * row;
        out[(top - 1 - d) * row..][..row].copy_from_slice(&inner[from..from + row]);
    }
    for d in 0..h - top - game_h {
        let from = (game_h - 1 - ping(d)) * row;
        out[(top + game_h + d) * row..][..row].copy_from_slice(&inner[from..from + row]);
    }
    Some(out)
}

/// The carousel ground, scaled to cover the whole screen, including behind the footer.
pub fn default_background() -> Option<&'static [u8]> {
    static P: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    P.get_or_init(|| art::cover_bytes(DEFAULT_PNG, OUT_W, OUT_H))
        .as_deref()
}

/// The blurred carousel ground, scaled to cover the whole screen the same way as the default.
pub fn blur_background() -> Option<&'static [u8]> {
    static P: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    P.get_or_init(|| art::cover_bytes(BLUR_PNG, OUT_W, OUT_H))
        .as_deref()
}

pub fn draw_ground(canvas: Canvas, face: Option<(TexId, u32)>, out: &mut Vec<Draw>) {
    let Some(face) = face else {
        return;
    };
    out.push(ground(canvas, face));
}

pub fn draw_backdrop(canvas: Canvas, face: Option<(TexId, u32)>, out: &mut Vec<Draw>) {
    let Some(face) = face else {
        return;
    };
    out.push(ground(canvas, face));
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: canvas.w,
        h: OUT_H as f32,
        colour: [0.0, 0.0, 0.0, SCRIM],
    });
}

fn ground(canvas: Canvas, (tex, tex_w): (TexId, u32)) -> Draw {
    Draw::Part {
        x: 0.0,
        y: 0.0,
        w: canvas.w,
        h: OUT_H as f32,
        tex,
        alpha: 1.0,
        uv: canvas.cover(tex_w as f32),
    }
}
