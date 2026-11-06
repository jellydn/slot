use slot::persist;
use slot_store::{Core, Platform, StateRing};

static LIVE_CORE: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn swan_is_opt_in_per_game_and_never_a_non_ps1_core() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    let selected =
        |stem, platform| slot_store::core_for_platform(root, stem, platform, Core::Gambatte);
    assert_eq!(selected("authored.cue", Platform::Ps), Core::PcsxRearmed);
    slot_store::write_selected_core(root, "authored.cue", Core::SwanStation).unwrap();
    assert_eq!(selected("authored.cue", Platform::Ps), Core::SwanStation);
    assert_eq!(selected("other.cue", Platform::Ps), Core::PcsxRearmed);
    assert_eq!(selected("authored.cue", Platform::Gba), Core::Mgba);
    assert_eq!(selected("authored.cue", Platform::Pico8), Core::Fake08);
    assert_eq!(Core::parse("SWANSTATION"), Some(Core::SwanStation));
}

#[test]
fn states_resume_and_cards_are_separate_and_never_automatically_migrated() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    let stem = "authored.cue";
    let pcsx = Core::PcsxRearmed;
    let swan = Core::SwanStation;
    let swan_card = vec![0x5a; 128 * 1024];
    persist::flush(
        root,
        Platform::Ps,
        pcsx,
        stem,
        Some(b"pcsx-state"),
        Some(b"pcsx-card"),
    )
    .unwrap();
    assert_eq!(
        persist::read_sav_for(root, Platform::Ps, swan, stem).unwrap(),
        None
    );
    assert_eq!(
        persist::read_resume(root, Platform::Ps, swan, stem).unwrap(),
        None
    );
    persist::flush(
        root,
        Platform::Ps,
        swan,
        stem,
        Some(b"swan-state"),
        Some(&swan_card),
    )
    .unwrap();
    for (core, state, card) in [
        (pcsx, b"pcsx-state", b"pcsx-card".as_slice()),
        (swan, b"swan-state", swan_card.as_slice()),
    ] {
        let ring = StateRing::new(root, Platform::Ps, core, stem);
        assert_eq!(ring.read_resume().unwrap().unwrap(), state);
        ring.push(state, b"thumb", "2026-11-06_00-00-00").unwrap();
        assert_eq!(ring.read("2026-11-06_00-00-00").unwrap().0, state);
        assert_eq!(
            persist::read_sav_for(root, Platform::Ps, core, stem)
                .unwrap()
                .unwrap(),
            card
        );
    }
    assert_eq!(
        persist::read_sav(root, Platform::Ps, stem)
            .unwrap()
            .unwrap(),
        b"pcsx-card"
    );
    assert_ne!(
        persist::core_save_dir_for(root, Platform::Ps, pcsx, stem),
        persist::core_save_dir_for(root, Platform::Ps, swan, stem)
    );
    assert_ne!(
        persist::core_save_dir_for(root, Platform::Ps, swan, stem),
        persist::core_save_dir_for(root, Platform::Ps, swan, "other.cue")
    );
}

#[test]
fn ps1_picker_can_select_swan_without_changing_other_pairs() {
    use slot::core_picker::{CorePicker, Outcome, Press};
    let mut picker = CorePicker::open(Core::pair(Platform::Ps).unwrap(), Core::PcsxRearmed, 0);
    picker.start(0);
    picker.press(Press::Right, 1000);
    assert_eq!(
        picker.press(Press::Keep, 2000),
        Outcome::Write(Core::SwanStation)
    );
    assert_eq!(Core::pair(Platform::Gba), Some([Core::Mgba, Core::Gpsp]));
}

#[test]
fn authored_contract_checks_software_options_card_directory_and_state_gate() {
    let _guard = LIVE_CORE.lock().unwrap();
    use slot_retro::ButtonMask;
    use slot_store::Blending;
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    slot::root::ensure(root);
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/swanstation.c");
    let library = root
        .join("System")
        .join(slot::core::dylib_name(Core::SwanStation));
    assert!(std::process::Command::new("cc")
        .args([
            if cfg!(target_os = "macos") {
                "-dynamiclib"
            } else {
                "-shared"
            },
            "-fPIC"
        ])
        .arg(source)
        .arg("-o")
        .arg(library)
        .status()
        .unwrap()
        .success());
    let rom = root.join("Games/PS/authored.chd");
    std::fs::write(&rom, b"authored contract input, not a real CHD").unwrap();
    let mut opened = slot::core::open_core(
        root,
        Core::SwanStation,
        Platform::Ps,
        &rom,
        "",
        false,
        Blending::Off,
        None,
        None,
    )
    .unwrap();
    assert!(opened.named);
    opened.core.load(&rom).unwrap();
    assert!(!opened.core.supports_states());
    assert!(opened.core.serialize().is_err());
    assert!(opened.core.unserialize(&[0; 16]).is_err());
    let card = vec![0x5a; 131072];
    opened.core.load_save_ram(&card).unwrap();
    opened.core.run_frame(ButtonMask::default());
    assert_eq!(opened.core.video_geometry().width, 320);
    assert_eq!(opened.core.video_xrgb8888().len(), 320 * 224 * 4);
    assert_eq!(opened.core.save_ram().unwrap(), card);
    assert!(opened.core.unserialize(&[0; 16]).is_err());
    assert_eq!(opened.core.save_ram().unwrap(), card);
}

#[test]
fn malformed_swan_cards_are_preserved_instead_of_partially_loaded_or_overwritten() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    let dir = persist::core_save_dir_for(root, Platform::Ps, Core::SwanStation, "authored.chd");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("memory.sav");
    std::fs::write(&path, b"truncated card").unwrap();
    assert!(persist::read_sav_for(root, Platform::Ps, Core::SwanStation, "authored.chd").is_err());
    assert!(persist::write_sav_for(
        root,
        Platform::Ps,
        Core::SwanStation,
        "authored.chd",
        &[0; 131072]
    )
    .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"truncated card");
}

#[test]
fn swan_session_reopen_preserves_cards_and_never_loads_foreign_resume() {
    use slot::audio::StubSink;
    use slot::session::Session;
    use slot_store::SlotState;
    use std::time::{Duration, Instant};
    let _guard = LIVE_CORE.lock().unwrap();
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    slot::root::ensure(root);
    let library = root
        .join("System")
        .join(slot::core::dylib_name(Core::SwanStation));
    assert!(std::process::Command::new("cc")
        .args([
            if cfg!(target_os = "macos") {
                "-dynamiclib"
            } else {
                "-shared"
            },
            "-fPIC"
        ])
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/swanstation.c"))
        .arg("-o")
        .arg(library)
        .status()
        .unwrap()
        .success());
    let name = "authored.chd";
    std::fs::write(root.join("Games/PS").join(name), b"authored contract input").unwrap();
    slot_store::write_selected_core(root, name, Core::SwanStation).unwrap();
    let card = vec![0x5a; 131072];
    persist::write_sav_for(root, Platform::Ps, Core::SwanStation, name, &card).unwrap();
    persist::write_sav_for(
        root,
        Platform::Ps,
        Core::PcsxRearmed,
        name,
        b"untouched PCSX card",
    )
    .unwrap();
    for core in [Core::PcsxRearmed, Core::SwanStation] {
        StateRing::new(root, Platform::Ps, core, name)
            .write_resume(b"must never reach unserialize")
            .unwrap();
    }
    for _ in 0..3 {
        slot_store::write_slot_state(
            root,
            &SlotState {
                cart: Some(name.into()),
                cart_platform: Some(Platform::Ps),
                clock_set: true,
                ..SlotState::default()
            },
        )
        .unwrap();
        let mut session = Session::boot_with(root.into(), Box::new(StubSink::draining()));
        let deadline = Instant::now() + Duration::from_secs(5);
        while session.frames_published() < 10 {
            session.update(0.02);
            assert!(
                Instant::now() < deadline,
                "Swan fixture did not boot through Session"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(session.frame().unwrap().geometry.width, 320);
        session.app_mut().flush_resume();
        drop(session);
        assert_eq!(
            persist::read_sav_for(root, Platform::Ps, Core::SwanStation, name)
                .unwrap()
                .unwrap(),
            card
        );
        assert_eq!(
            persist::read_sav(root, Platform::Ps, name)
                .unwrap()
                .unwrap(),
            b"untouched PCSX card"
        );
        for core in [Core::PcsxRearmed, Core::SwanStation] {
            assert_eq!(
                StateRing::new(root, Platform::Ps, core, name)
                    .read_resume()
                    .unwrap()
                    .unwrap(),
                b"must never reach unserialize"
            );
        }
    }
}
