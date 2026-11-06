#![cfg(target_os = "macos")]

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use slot::thumb;
use slot_gfx::{lcd3x_mask, Compositor, Draw, HeadlessSurface, OUT_W, SRC_H, SRC_W};
use slot_retro::{ButtonMask, MockCore, RetroCore};
use slot_store::{Core, Platform, StateRing};
use slot_ui::{photo_face, Polaroids, Printed};

static GL: Mutex<()> = Mutex::new(());

fn compositor() -> Option<(MutexGuard<'static, ()>, HeadlessSurface, Compositor)> {
    let guard = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((guard, surface, compositor))
}

fn px(frame: &[u8], x: usize, y: usize) -> [u8; 3] {
    let o = (y * OUT_W as usize + x) * 4;
    [frame[o], frame[o + 1], frame[o + 2]]
}

fn switcher_photo(p: &Polaroids) -> Vec<Draw> {
    let mut out = Vec::new();
    p.draw(None, Printed::default(), None, Printed::default(), &mut out);
    let photo = *out.first().expect("the switcher drew nothing");
    assert!(
        matches!(photo, Draw::Shot { .. }),
        "the screenshot is not drawn on the panel"
    );
    vec![photo]
}

fn mock_frame(frames: u32) -> Vec<u8> {
    let mut core = MockCore::new();
    core.load(Path::new("mock")).expect("mock load");
    for _ in 0..frames {
        core.run_frame(ButtonMask::default());
    }
    core.video_xrgb8888().to_vec()
}

#[test]
fn a_mock_frame_keeps_its_colours_through_the_game_pass() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let src = mock_frame(7);

    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let frame = c.read_frame();

    let mask = lcd3x_mask();
    let mut worst = 0i32;
    for (sx, sy) in [(0, 0), (5, 1), (113, 37), (120, 80), (239, 159)] {
        let o = (sy * SRC_W as usize + sx) * 4;
        let want_rgb = [src[o + 2], src[o + 1], src[o]];
        for (oy, row) in mask.iter().enumerate() {
            for (ox, cell) in row.iter().enumerate() {
                let got = px(&frame, sx * 3 + ox, sy * 3 + oy);
                for (ch, gain) in cell.iter().enumerate() {
                    let want = (want_rgb[ch] as f32 * gain).round() as i32;
                    worst = worst.max((want - got[ch] as i32).abs());
                }
            }
        }
    }
    assert!(
        worst <= 1,
        "max channel deviation {worst} from the mock frame"
    );
}

#[test]
fn native_video_resizes_without_cropping_and_uses_display_aspect() {
    let _g = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().expect("native video test needs OpenGL");
    let mut c = Compositor::new(&surface).unwrap();
    c.set_screen_effect(slot_gfx::ScreenEffect::None);
    c.set_game_source_rect([0.05, 0.1, 0.8, 0.7]);
    c.set_picture([0.05, 0.1, 0.85, 0.8]);
    // 320×448 is deliberately not a square-pixel 4:3 frame. Shrinking afterwards
    // must replace the texture, not expose stale pixels from the previous frame.
    for (name, width, height, aspect, left, right) in [
        ("ps1-native", 320, 448, 4.0 / 3.0, 40, 679),
        ("pico8-native", 128, 128, 1.0, 120, 599),
    ] {
        let mut source = Vec::new();
        for y in 0..height {
            for x in 0..width {
                source.extend_from_slice(match (x < width / 2, y < height / 2) {
                    (true, true) => &[0, 0, 255, 255],
                    (false, true) => &[0, 255, 0, 255],
                    (true, false) => &[255, 0, 0, 255],
                    (false, false) => &[0, 255, 255, 255],
                });
            }
        }
        c.begin_frame();
        assert!(c.upload_native_game(&source, (width, height), aspect));
        c.draw_game();
        let frame = c.read_frame();
        for (x, y, expected) in [
            (left + 1, 1, [255, 0, 0]),
            (right - 1, 1, [0, 255, 0]),
            (left + 1, 478, [0, 0, 255]),
            (right - 1, 478, [255, 255, 0]),
            (left - 1, 240, [0, 0, 0]),
            (right + 1, 240, [0, 0, 0]),
        ] {
            assert_eq!(px(&frame, x, y), expected, "{name} at {x},{y}");
        }
        let rgba: Vec<u8> = source
            .chunks_exact(4)
            .flat_map(|p| [p[2], p[1], p[0], 255])
            .collect();
        let tex = c.create_texture_nearest(width, height, &rgba);
        c.begin_frame();
        c.draw_list(&[Draw::Shot { tex }]);
        assert!(
            c.read_frame() == frame,
            "a native still must not use a GB crop"
        );
        assert!(!c.upload_native_game(&source[..source.len() - 1], (width, height), aspect));
        assert!(!c.upload_native_game(&source, (width, height), f32::NAN));
        assert!(!c.upload_native_game(&source, (u32::MAX, height), aspect));
        c.begin_frame();
        c.draw_game();
        assert!(
            c.read_frame() == frame,
            "invalid frames preserve the last good picture"
        );
        if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
            let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
            let mut encoder = png::Encoder::new(file, 720, 480);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&frame)
                .unwrap();
        }
    }
    c.set_game_source_rect(slot_gfx::WHOLE_TEXTURE);
    c.set_picture(slot_gfx::WHOLE_TEXTURE);
    let src = mock_frame(7);
    c.begin_frame();
    c.upload_game(&src);
    c.draw_game();
    let frame = c.read_frame();
    for (sx, sy) in [(0, 0), (113, 37), (239, 159)] {
        let o = (sy * 240 + sx) * 4;
        assert_eq!(
            px(&frame, sx * 3 + 1, sy * 3 + 1),
            [src[o + 2], src[o + 1], src[o]],
            "returning to GB restores its canvas and aspect"
        );
    }
}

#[test]
fn native_bezels_stay_behind_every_pixel_of_the_picture() {
    let _g = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().expect("native bezel test needs OpenGL");
    let mut c = Compositor::new(&surface).unwrap();
    c.set_screen_effect(slot_gfx::ScreenEffect::None);
    for (name, platform, size, aspect) in [
        ("ps1", Platform::Ps, (320, 448), 4.0 / 3.0),
        ("pico8", Platform::Pico8, (128, 128), 1.0),
    ] {
        let pixels = [17, 83, 229, 255].repeat(size.0 as usize * size.1 as usize);
        for panel in [(720, 480), (640, 480), (1280, 720)] {
            c.fit(panel);
            c.set_game_bezel(None);
            c.begin_frame();
            assert!(c.upload_native_game(&pixels, size, aspect));
            c.draw_list(&[Draw::Game]);
            let plain = c.read_frame();
            for (kind, background) in [
                (
                    "default",
                    slot_ui::native_bezel_face(platform, panel.0, panel.1),
                ),
                (
                    "custom",
                    [61, 38, 73, 255].repeat(panel.0 as usize * panel.1 as usize),
                ),
            ] {
                let tex = c.create_texture(panel.0, panel.1, &background);
                c.set_game_bezel(Some(tex));
                c.begin_frame();
                c.draw_list(&[Draw::Game]);
                let framed = c.read_frame();
                let mut picture = 0;
                let mut border = 0;
                for (before, after) in plain.chunks_exact(4).zip(framed.chunks_exact(4)) {
                    if before[..3] != [0, 0, 0] {
                        assert_eq!(before, after, "{name} {kind} {panel:?} picture changed");
                        picture += 1;
                    } else {
                        assert_ne!(after[..3], [0, 0, 0], "border must be visible");
                        border += 1;
                    }
                }
                assert!(picture > 100_000 && border > 1_000);
                if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
                    let path = format!("{dir}/{name}-bezel-{kind}-{}.png", panel.0);
                    let mut e =
                        png::Encoder::new(std::fs::File::create(path).unwrap(), panel.0, panel.1);
                    e.set_color(png::ColorType::Rgba);
                    e.write_header().unwrap().write_image_data(&framed).unwrap();
                }
            }
        }
    }
}

#[test]
fn a_saved_frame_arrives_intact_on_its_screenshot() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let src = mock_frame(13);
    let d = tempfile::tempdir().expect("tempdir");
    let ring = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Mock");
    let stamp = "2026-08-09_14-32-05";
    ring.push(b"state", &thumb::png(&src).expect("encode"), stamp)
        .expect("push");
    let entries = ring.list().expect("list");

    let face = photo_face(&entries[0]);
    let tex = c.create_texture(face.w, face.h, &face.rgba);
    c.begin_frame();
    c.draw_list(&[Draw::Tex {
        x: 0.0,
        y: 0.0,
        w: face.w as f32,
        h: face.h as f32,
        tex,
        alpha: 1.0,
    }]);
    let frame = c.read_frame();

    for (sx, sy) in [(0, 0), (113, 37), (239, 159)] {
        let o = (sy * SRC_W as usize + sx) * 4;
        let want = [src[o + 2], src[o + 1], src[o]];
        let got = px(&frame, sx, sy);
        assert_eq!(got, want, "screenshot pixel {sx},{sy}");
    }
}

#[test]
fn the_switcher_magnifies_its_screenshot_without_resampling_it() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let src = mock_frame(21);
    let d = tempfile::tempdir().expect("tempdir");
    let ring = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Mock");
    ring.push(
        b"state",
        &thumb::png(&src).expect("encode"),
        "2026-08-09_14-32-05",
    )
    .expect("push");
    let entries = ring.list().expect("list");

    let face = photo_face(&entries[0]);
    let tex = c.create_texture_nearest(face.w, face.h, &face.rgba);
    let mut switcher = Polaroids::new(entries);
    switcher.set_faces(vec![tex]);
    c.set_screen_power(1.0);
    c.begin_frame();
    c.draw_list(&switcher_photo(&switcher));
    let frame = c.read_frame();

    let mask = lcd3x_mask();
    for (sx, sy) in [
        (0, 0),
        (1, 0),
        (113, 37),
        (SRC_W as usize - 1, SRC_H as usize - 1),
    ] {
        let o = (sy * SRC_W as usize + sx) * 4;
        let want_rgb = [src[o + 2], src[o + 1], src[o]];
        for (oy, row) in mask.iter().enumerate() {
            for (ox, cell) in row.iter().enumerate() {
                let got = px(&frame, sx * 3 + ox, sy * 3 + oy);
                for (ch, gain) in cell.iter().enumerate() {
                    let want = (want_rgb[ch] as f32 * gain).round() as i32;
                    assert!(
                        (want - got[ch] as i32).abs() <= 1,
                        "source pixel {sx},{sy} bled into its neighbours"
                    );
                }
            }
        }
    }
}
