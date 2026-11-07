mod common;

use common::tmp_root;
use slot_store::{scan, Platform};
use tempfile::TempDir;

fn write_rom(d: &TempDir, rel: &str, title: &str) {
    let mut rom = vec![0u8; 0x100];
    rom[0xa0..0xa0 + title.len()].copy_from_slice(title.as_bytes());
    std::fs::write(d.path().join("Games").join(rel), rom).expect("write rom");
}

fn write_png(d: &TempDir, rel: &str) {
    let path = d.path().join("Labels").join(rel);
    std::fs::create_dir_all(path.parent().expect("label has a parent")).expect("create dir");
    std::fs::write(path, b"\x89PNG\r\n\x1a\n").expect("write png");
}

#[test]
fn a_png_in_labels_is_paired_to_its_rom_by_stem() {
    let d = tmp_root();
    write_rom(&d, "GBA/Pokemon Emerald.gba", "POKEMON EMER");
    write_png(&d, "GBA/Pokemon Emerald.png");
    write_rom(&d, "GBA/Advance Wars.gba", "ADVANCEWARS");
    let carts = scan(d.path()).unwrap();
    assert_eq!(carts.len(), 2);
    assert_eq!(carts[0].stem, "Advance Wars");
    assert!(carts[0].label.is_none());
    assert!(
        carts[1].label.is_some(),
        "a label in Labels/ was not picked up"
    );
    let mut emerald = carts[1].clone();
    emerald.read_header();
    assert_eq!(emerald.title, "POKEMON EMER");
}

#[test]
fn a_label_pairs_with_its_rom_whatever_the_case_of_its_name() {
    let d = tmp_root();
    write_rom(&d, "GBA/Pokemon Emerald.gba", "POKEMON EMER");
    write_png(&d, "GBA/pokemon emerald.PNG");
    let carts = scan(d.path()).unwrap();
    assert_eq!(
        carts[0].label.as_deref(),
        Some(d.path().join("Labels/GBA/pokemon emerald.PNG").as_path())
    );
}

#[test]
fn a_scan_reads_names_and_leaves_the_rom_headers_for_later() {
    let d = tmp_root();
    write_rom(&d, "GBA/Advance Wars.gba", "ADVANCEWARS");
    let mut cart = scan(d.path()).unwrap().remove(0);
    assert!(cart.title.is_empty());
    cart.read_header();
    assert_eq!(cart.title, "ADVANCEWARS");
}

#[test]
fn scan_ignores_non_gba_files() {
    let d = tmp_root();
    write_rom(&d, "GBA/Real.gba", "REAL");
    std::fs::write(d.path().join("Games/GBA/notes.txt"), "hi").unwrap();
    assert_eq!(scan(d.path()).unwrap().len(), 1);
}

#[test]
fn an_appledouble_sidecar_is_not_shelved_as_a_cart() {
    let d = tmp_root();
    write_rom(&d, "GBA/Metroid Fusion.gba", "METROID");
    write_rom(&d, "GBA/._Metroid Fusion.gba", "METROID");
    let carts = scan(d.path()).unwrap();
    assert_eq!(carts.len(), 1, "an AppleDouble sidecar reached the shelf");
    assert_eq!(carts[0].stem, "Metroid Fusion");
}

#[test]
fn header_title_of_a_truncated_rom_is_none_not_a_panic() {
    let d = tmp_root();
    std::fs::write(d.path().join("Games/GBA/Tiny.gba"), [0u8; 8]).unwrap();
    let mut cart = scan(d.path()).unwrap().remove(0);
    cart.read_header();
    assert!(cart.title.is_empty());
}

#[test]
fn a_header_title_that_is_not_text_is_dropped_rather_than_mangled() {
    let d = tmp_root();
    let mut rom = vec![0u8; 0x100];
    rom[0xa0..0xac].copy_from_slice(&[0xffu8; 12]);
    std::fs::write(d.path().join("Games/GBA/Garbage.gba"), rom).unwrap();
    let mut cart = scan(d.path()).unwrap().remove(0);
    cart.read_header();
    assert!(cart.title.is_empty());
}

#[test]
fn a_root_with_no_games_directory_scans_as_empty() {
    let d = tempfile::tempdir().unwrap();
    assert!(scan(d.path()).unwrap().is_empty());
}

#[test]
fn a_cart_takes_the_platform_of_the_folder_it_is_in() {
    let d = tmp_root();
    write_rom(&d, "GBA/Metroid Fusion.gba", "METROID");
    std::fs::write(d.path().join("Games/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    std::fs::write(d.path().join("Games/GBC/Chromatic.gbc"), vec![0u8; 0x150]).unwrap();

    let carts = scan(d.path()).unwrap();

    let by_stem = |s: &str| carts.iter().find(|c| c.stem == s).unwrap().platform;
    assert_eq!(by_stem("Metroid Fusion"), Platform::Gba);
    assert_eq!(by_stem("Tetris"), Platform::Gb);
    assert_eq!(by_stem("Chromatic"), Platform::Gbc);
}

#[test]
fn a_gba_rom_in_the_game_boy_folder_does_not_appear() {
    let d = tmp_root();
    std::fs::write(d.path().join("Games/GB/Wrong.gba"), vec![0u8; 0x150]).unwrap();
    assert!(scan(d.path()).unwrap().is_empty());
}

#[test]
fn an_unreadable_platform_folder_does_not_take_the_rest_of_the_library_with_it() {
    let d = tmp_root();
    write_rom(&d, "GBA/Metroid Fusion.gba", "METROID");
    std::fs::write(d.path().join("Games/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    std::fs::remove_dir(d.path().join("Games/GBC")).unwrap();
    std::fs::write(d.path().join("Games/GBC"), b"not a directory").unwrap();

    let carts = scan(d.path()).expect("one bad folder must not fail the whole scan");

    assert_eq!(carts.len(), 2, "the readable shelves were lost too");
    assert!(carts.iter().any(|c| c.stem == "Metroid Fusion"));
    assert!(carts.iter().any(|c| c.stem == "Tetris"));
}

#[test]
fn the_same_stem_on_two_platforms_is_two_carts_with_two_labels() {
    let d = tmp_root();
    write_rom(&d, "GBA/Tetris.gba", "TETRIS");
    std::fs::write(d.path().join("Games/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    std::fs::create_dir_all(d.path().join("Labels/GB")).unwrap();
    std::fs::write(d.path().join("Labels/GB/Tetris.png"), b"\x89PNG\r\n\x1a\n").unwrap();

    let carts = scan(d.path()).unwrap();

    assert_eq!(carts.len(), 2);
    let gb = carts.iter().find(|c| c.platform == Platform::Gb).unwrap();
    let gba = carts.iter().find(|c| c.platform == Platform::Gba).unwrap();
    assert!(gb.label.is_some(), "the Game Boy label was not found");
    assert!(
        gba.label.is_none(),
        "the GBA cart borrowed the Game Boy label"
    );
}

#[test]
fn a_valid_ps1_playlist_is_one_cart_and_native_formats_keep_distinct_identities() {
    let d = tmp_root();
    let ps = d.path().join("Games/PS");
    let pico = d.path().join("Games/PICO8");
    std::fs::create_dir_all(&ps).unwrap();
    std::fs::create_dir_all(&pico).unwrap();
    for name in ["First.chd", "Second.pbp", "Solo.chd", "Solo.pbp"] {
        std::fs::write(ps.join(name), b"authored").unwrap();
    }
    std::fs::write(ps.join("Set.m3u"), "# discs\nFirst.chd\nSecond.pbp\n").unwrap();
    std::fs::write(pico.join("Cart.p8"), b"text").unwrap();
    std::fs::write(pico.join("Cart.p8.png"), b"encoded").unwrap();
    let carts = scan(d.path()).unwrap();
    let names: Vec<_> = carts.iter().map(|c| c.stem.as_str()).collect();
    assert_eq!(
        names,
        ["Set.m3u", "Solo.chd", "Solo.pbp", "Cart.p8", "Cart.p8.png"]
    );
    assert_eq!(
        slot_store::disc::contents(&ps, &ps.join("Set.m3u")).unwrap(),
        [ps.join("First.chd"), ps.join("Second.pbp")]
    );
    std::fs::write(ps.join("Set.m3u"), "First.chd\nMissing.chd\n").unwrap();
    assert_eq!(
        scan(d.path()).unwrap().len(),
        7,
        "invalid playlists must not hide usable discs"
    );
}

#[test]
fn ps1_disc_validation_bounds_descriptors_and_rejects_missing_or_escaping_members() {
    let d = tmp_root();
    let ps = d.path().join("Games/PS");
    std::fs::create_dir_all(&ps).unwrap();
    std::fs::write(ps.join("track.bin"), b"authored track").unwrap();
    let cue = ps.join("Disc.cue");
    std::fs::write(
        &cue,
        "FILE \"track.bin\" BINARY\n TRACK 01 MODE1/2048\n INDEX 01 00:00:00\n",
    )
    .unwrap();
    assert_eq!(
        slot_store::disc::contents(&ps, &cue).unwrap(),
        std::slice::from_ref(&cue)
    );
    for descriptor in [
        "FILE \"missing.bin\" BINARY\n",
        "FILE \"../outside.bin\" BINARY\n",
        "FILE \"/outside.bin\" BINARY\n",
        "FILE \"..\\outside.bin\" BINARY\n",
        "TRACK 01 MODE1/2048\n",
    ] {
        std::fs::write(&cue, descriptor).unwrap();
        assert!(
            slot_store::disc::contents(&ps, &cue).is_err(),
            "{descriptor}"
        );
    }
    std::fs::write(&cue, vec![b' '; 65537]).unwrap();
    assert!(slot_store::disc::contents(&ps, &cue).is_err());
    let playlist = ps.join("Set.m3u");
    for descriptor in ["", "Set.m3u\n", "Other.m3u\n"] {
        std::fs::write(&playlist, descriptor).unwrap();
        assert!(slot_store::disc::contents(&ps, &playlist).is_err());
    }
    std::fs::write(ps.join("One.chd"), b"authored").unwrap();
    std::fs::write(&playlist, "One.chd\n".repeat(15)).unwrap();
    assert_eq!(
        slot_store::disc::contents(&ps, &playlist).unwrap().len(),
        15
    );
    std::fs::write(&playlist, "One.chd\n".repeat(16)).unwrap();
    assert!(slot_store::disc::contents(&ps, &playlist).is_err());
    let outside = d.path().join("outside.bin");
    std::fs::write(&outside, b"outside").unwrap();
    std::os::unix::fs::symlink(&outside, ps.join("escape.bin")).unwrap();
    std::fs::write(&cue, "FILE \"escape.bin\" BINARY\n").unwrap();
    assert!(slot_store::disc::contents(&ps, &cue).is_err());
    // A symlink's extension must not bypass parsing of the name the core sees.
    std::os::unix::fs::symlink(ps.join("One.chd"), ps.join("Alias.m3u")).unwrap();
    assert!(slot_store::disc::contents(&ps, &ps.join("Alias.m3u")).is_err());
}
