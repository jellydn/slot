use std::path::Path;
use std::time::{Duration, Instant};

use slot::art::index::ArtIndex;
use slot::art::jobs::{art_jobs, fetch_missing_with, studio_art_with, ArtEvent, ArtJob};
use slot::art::make::Kind;
use slot::art::pico8::{read_label, LabelError};
use slot_store::Platform;

fn text() -> Vec<u8> {
    let mut s = String::from("\u{feff}pico-8 cartridge // http://www.pico-8.com\r\nversion 42\r\n__lua__\r\n#include absent.p8\r\n__label__\r\n");
    for y in 0..128 {
        s.push_str(&if y == 0 { "8" } else { "g" }.repeat(127));
        s.push('v');
        s.push_str("\r\n");
    }
    s.push_str("__gfx__\r\n0000\r\n");
    s.into_bytes()
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut pixels = vec![0; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            pixels[i..i + 4].copy_from_slice(&[x as u8, y as u8, 61, 0]);
        }
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    out
}

fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut r = png::Decoder::new(bytes).read_info().unwrap();
    let mut pixels = vec![0; r.output_buffer_size()];
    let frame = r.next_frame(&mut pixels).unwrap();
    (frame.width, frame.height, pixels)
}

fn put(root: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let p = root.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn text_label_decodes_base_and_extended_colours_without_following_includes() {
    let d = tempfile::tempdir().unwrap();
    let p = put(d.path(), "Authored.P8", &text());
    let (w, h, pixels) = decode(&read_label(&p).unwrap());
    assert_eq!((w, h), (128, 128));
    assert_eq!(&pixels[..3], &[255, 0, 77]);
    assert_eq!(&pixels[128 * 3..128 * 3 + 3], &[41, 24, 20]);
    assert_eq!(&pixels[127 * 3..128 * 3], &[255, 157, 129]);
}

#[test]
fn png_uses_exact_visible_rectangle_and_ignores_payload_alpha() {
    let d = tempfile::tempdir().unwrap();
    let p = put(d.path(), "Renamed.p8.png", &png(160, 205));
    let (w, h, pixels) = decode(&read_label(&p).unwrap());
    assert_eq!((w, h), (128, 128));
    assert_eq!(&pixels[..3], &[16, 24, 61]);
    assert_eq!(&pixels[(127 * 128 + 127) * 3..], &[143, 151, 61]);
    assert_eq!(
        &pixels[(63 * 128 + 19) * 3..(63 * 128 + 19) * 3 + 3],
        &[35, 87, 61]
    );
}

#[test]
fn invalid_missing_and_unsupported_labels_have_distinct_bounded_results() {
    let d = tempfile::tempdir().unwrap();
    let header = b"pico-8 cartridge // http://www.pico-8.com\nversion 42\n__lua__\nprint('hi')\n";
    for (name, data, error) in [
        ("Missing.p8", header.to_vec(), LabelError::Missing),
        (
            "Broken.p8",
            b"not a cartridge\n__label__\n".to_vec(),
            LabelError::Malformed,
        ),
        (
            "Short.p8",
            [header.as_slice(), b"__label__\n0\n"].concat(),
            LabelError::Malformed,
        ),
        (
            "BadColour.p8",
            String::from_utf8(text())
                .unwrap()
                .replace(&"g".repeat(127), &"w".repeat(127))
                .into_bytes(),
            LabelError::Malformed,
        ),
        (
            "Large.p8",
            vec![0; 2 * 1024 * 1024 + 1],
            LabelError::TooLarge,
        ),
        ("WrongSize.p8.png", png(205, 160), LabelError::Unsupported),
        ("Broken.p8.png", b"not PNG".to_vec(), LabelError::Malformed),
        ("Plain.png", png(160, 205), LabelError::Unsupported),
    ] {
        let path = put(d.path(), name, &data);
        assert_eq!(read_label(&path), Err(error), "{name}");
    }
    let mut truncated = png(160, 205);
    truncated.truncate(truncated.len() - 10);
    assert_eq!(
        read_label(&put(d.path(), "Truncated.p8.png", &truncated)),
        Err(LabelError::Malformed)
    );
    let duplicate = [text(), b"__label__\n".to_vec()].concat();
    assert_eq!(
        read_label(&put(d.path(), "Duplicate.p8", &duplicate)),
        Err(LabelError::Malformed)
    );
}

#[test]
fn real_worker_extracts_both_formats_offline_and_keeps_existing_labels() {
    let d = tempfile::tempdir().unwrap();
    for (name, bytes) in [
        ("Text.p8", text()),
        ("Image.p8.png", png(160, 205)),
        ("Custom.p8", text()),
    ] {
        put(d.path(), &format!("Games/PICO8/{name}"), &bytes);
    }
    put(
        d.path(),
        "Games/PICO8/Missing.p8",
        b"pico-8 cartridge // http://www.pico-8.com\nversion 42\n__lua__\n",
    );
    let custom = put(
        d.path(),
        "Labels/PICO8/Custom.p8.png",
        b"keep exact custom bytes",
    );
    let mut jobs = art_jobs(d.path().to_path_buf());
    jobs.ask(ArtJob::FetchMissing(Kind::Logo));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    loop {
        assert!(
            Instant::now() < deadline,
            "worker did not finish: {events:?}"
        );
        if let Some(event) = jobs.take() {
            let done = matches!(event, ArtEvent::Done { .. } | ArtEvent::Failed(_));
            events.push(event);
            if done {
                break;
            }
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert!(matches!(events.first(), Some(ArtEvent::Counted(3))));
    assert!(
        matches!(events.last(), Some(ArtEvent::Done { landed: 2, missed }) if missed == &["Missing.p8"])
    );
    for name in ["Text.p8", "Image.p8.png"] {
        let path = d.path().join(format!("Labels/PICO8/{name}.png"));
        let (w, h, _) = decode(&std::fs::read(path).unwrap());
        assert_eq!((w, h), (128, 128));
        let cart = slot_store::scan(d.path())
            .unwrap()
            .into_iter()
            .find(|c| c.stem == name)
            .unwrap();
        assert!(slot::label_cache::cache_path(d.path(), &cart).is_file());
    }
    assert_eq!(std::fs::read(custom).unwrap(), b"keep exact custom bytes");
    assert!(!d.path().join("Cache/art/index.json").exists());
}

#[test]
fn mixed_library_keeps_nintendo_fetching_and_local_studio_does_not_replace_custom_art() {
    let d = tempfile::tempdir().unwrap();
    put(d.path(), "Games/PICO8/Text.p8", &text());
    put(d.path(), "Games/GBA/Game.gba", &[42; 512]);
    let crc = crc32fast::hash(&[42; 512]);
    let index = ArtIndex::parse(&format!(
        r#"{{"{crc:08X}":{{"support-texture":"GBA/test.png"}}}}"#
    ))
    .unwrap();
    let mut events = Vec::new();
    fetch_missing_with(
        d.path(),
        Kind::Real,
        &index,
        &|url| {
            assert!(url.ends_with("GBA/test.webp") || url.ends_with("GBA/test.png"));
            Ok(png(160, 205))
        },
        &|| false,
        &mut |e| events.push(e),
    );
    assert!(
        matches!(events.last(), Some(ArtEvent::Done { landed: 2, missed }) if missed.is_empty())
    );
    let cart = slot_store::scan(d.path())
        .unwrap()
        .into_iter()
        .find(|c| c.platform == Platform::Pico8)
        .unwrap();
    let label = cart.label.as_ref().unwrap();
    std::fs::write(label, b"custom").unwrap();
    let studio = studio_art_with(d.path(), &cart, &index, &mut |_| {
        panic!("local studio used network")
    });
    assert!(studio.real.is_some());
    assert_eq!(std::fs::read(label).unwrap(), b"custom");
    assert_eq!(slot::art::jobs::fetch_label(d.path(), &cart), Ok(true));
    assert_eq!(std::fs::read(label).unwrap(), b"custom");
}
