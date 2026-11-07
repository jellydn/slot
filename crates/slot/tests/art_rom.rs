use slot::art::rom::{missing_art, rom_crc};

#[test]
fn crc_of_known_bytes() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("check.bin");
    std::fs::write(&p, b"123456789").unwrap();
    assert_eq!(rom_crc(&p).unwrap(), 0xCBF43926);
}

#[test]
fn crc_streams_a_large_file() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("big.gba");
    let bytes = vec![0xA5u8; 3 * 1024 * 1024 + 17];
    std::fs::write(&p, &bytes).unwrap();
    assert_eq!(rom_crc(&p).unwrap(), crc32fast::hash(&bytes));
}

#[test]
fn only_carts_without_labels_are_missing() {
    let d = tempfile::tempdir().unwrap();
    for dir in ["Games/GBA", "Labels/GBA"] {
        std::fs::create_dir_all(d.path().join(dir)).unwrap();
    }
    std::fs::write(d.path().join("Games/GBA/A.gba"), vec![0u8; 0x200]).unwrap();
    std::fs::write(d.path().join("Games/GBA/B.gba"), vec![0u8; 0x200]).unwrap();
    std::fs::write(d.path().join("Labels/GBA/B.png"), b"png").unwrap();
    let stems: Vec<String> = missing_art(d.path()).into_iter().map(|c| c.stem).collect();
    assert_eq!(stems, vec!["A".to_string()]);
}

#[test]
fn native_labels_keep_the_complete_game_filename() {
    use slot::art::make::{label_path, relabel};
    use slot_store::Platform;

    let d = tempfile::tempdir().unwrap();
    let mut png = Vec::new();
    {
        let encoder = png::Encoder::new(&mut png, 1, 1);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[90])
            .unwrap();
    }
    for (platform, name) in [
        (Platform::Ps, "Disc.chd"),
        (Platform::Ps, "Disc.pbp"),
        (Platform::Ps, "Tracks.cue"),
        (Platform::Pico8, "Cart.p8"),
        (Platform::Pico8, "Cart.p8.png"),
    ] {
        let games = d.path().join("Games").join(platform.dir_name());
        std::fs::create_dir_all(&games).unwrap();
        std::fs::write(games.join(name), b"authored filename fixture").unwrap();
        let cart = missing_art(d.path())
            .into_iter()
            .find(|c| c.platform == platform && c.stem == name)
            .unwrap();
        let expected = d
            .path()
            .join("Labels")
            .join(platform.dir_name())
            .join(format!("{name}.png"));
        assert_eq!(label_path(d.path(), &cart), expected);
        relabel(d.path(), &cart, &png).unwrap();
        let scanned = slot_store::scan(d.path()).unwrap();
        let labeled = scanned
            .iter()
            .find(|c| c.platform == platform && c.stem == name)
            .unwrap();
        assert_eq!(labeled.label.as_ref(), Some(&expected));
        assert!(!missing_art(d.path())
            .iter()
            .any(|c| c.platform == platform && c.stem == name));
    }
}
