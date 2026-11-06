use std::path::Path;
use std::time::{Duration, Instant};

use slot::audio::StubSink;
use slot::session::Session;
use slot_store::{Core, Platform, SlotState, StateRing};

fn frame(session: &mut Session, width: u32) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        session.update(0.02);
        if session.game_visible() && session.frames_published() >= 10 {
            if let Some(frame) = session.frame() {
                assert_eq!(frame.geometry.width, width);
                if width == 128 {
                    assert_eq!(frame.geometry.height, 128);
                } else {
                    assert!((224..=240).contains(&frame.geometry.height));
                }
                return frame.to_vec();
            }
        }
        assert!(
            Instant::now() < deadline,
            "native game did not produce a frame"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "future fake08 quick-save/resume repair gate; requires SLOT_FAKE08_CORE"]
fn fake08_slot_quick_save_and_resume_restore_lua_gameplay_after_reopen() {
    use slot_input::{Action, Btn, RawEvent};
    let library = std::env::var_os("SLOT_FAKE08_CORE").expect("set SLOT_FAKE08_CORE");
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    slot::root::ensure(root);
    std::fs::copy(
        library,
        root.join("System")
            .join(slot::core::dylib_name(Core::Fake08)),
    )
    .unwrap();
    let name = "quick-state.p8";
    let mut text = include_str!("../../slot-retro/tests/fixtures/quick-state.p8").to_owned();
    // Enough nested prototypes to trigger GC while Eris rebuilds a fresh VM.
    for i in 0..256 {
        text.push_str(&format!(
            "function f{i}() return function() return 'constant{i}' end end\n"
        ));
    }
    std::fs::write(root.join("Games/PICO8").join(name), &text).unwrap();
    // Single-cart libraries intentionally hide/refuse Exit Game.
    std::fs::write(root.join("Games/PICO8/zz-other.p8"), text).unwrap();
    slot_store::write_slot_state(
        root,
        &SlotState {
            cart: Some(name.into()),
            cart_platform: Some(Platform::Pico8),
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let mut first = Session::boot_with(root.into(), Box::new(StubSink::draining()));
    let initial = frame(&mut first, 128);
    first.feed([RawEvent::Down(Btn::Left)], first.app().now());
    let deadline = Instant::now() + Duration::from_secs(3);
    while frame(&mut first, 128) == initial {
        assert!(
            Instant::now() < deadline,
            "authored Lua input did not advance gameplay"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    first.feed([RawEvent::Up(Btn::Left)], first.app().now());
    let released_at = first.frames_published();
    let deadline = Instant::now() + Duration::from_secs(3);
    while first.frames_published() < released_at + 3 {
        first.update(0.02);
        assert!(
            Instant::now() < deadline,
            "released input did not reach the core"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let expected = frame(&mut first, 128);
    first.app_mut().apply(Action::SaveState);
    for _ in 0..3 {
        first.feed([RawEvent::Down(Btn::Left)], first.app().now());
        let deadline = Instant::now() + Duration::from_secs(3);
        while frame(&mut first, 128) == expected {
            assert!(
                Instant::now() < deadline,
                "gameplay did not change after save"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        first.feed([RawEvent::Up(Btn::Left)], first.app().now());
        let released_at = first.frames_published();
        while first.frames_published() < released_at + 3 {
            first.update(0.02);
            assert!(
                Instant::now() < deadline,
                "input release did not reach the core"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        first.app_mut().apply(Action::LoadState);
        while frame(&mut first, 128) != expected {
            assert!(
                Instant::now() < deadline,
                "Slot quick load did not restore Lua gameplay"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    first.app_mut().flush_resume();
    drop(first);
    let ring = StateRing::new(root, Platform::Pico8, Core::Fake08, name);
    assert!(
        ring.has_resume(),
        "Slot must write a genuine resume state, not cartdata alone"
    );
    assert!(
        !ring.list().unwrap().is_empty(),
        "quick save must also create a manual state"
    );
    let mut reopened = Session::boot_with(root.into(), Box::new(StubSink::draining()));
    assert!(
        frame(&mut reopened, 128) == expected,
        "Slot resume must restore Lua closure/table state instead of restarting the cart"
    );
    for _ in 0..3 {
        reopened.app_mut().apply(Action::Eject);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !matches!(reopened.app().phase(), slot::app::Phase::Shelf) {
            reopened.update(0.02);
            assert!(
                Instant::now() < deadline,
                "Exit Game did not return to shelf"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        reopened.app_mut().apply(Action::Insert);
        assert!(
            frame(&mut reopened, 128) == expected,
            "Exit Game and reinsert must restore the saved Lua gameplay"
        );
    }
}

#[test]
#[ignore = "requires SLOT_FAKE08_CORE built with cores/ps1-pico8/build.sh"]
fn text_and_png_carts_preserve_cartdata_after_refusing_an_invalid_resume() {
    let library = std::env::var_os("SLOT_FAKE08_CORE").expect("set SLOT_FAKE08_CORE");
    let text = include_str!("../../slot-retro/tests/fixtures/persistence.p8");
    for name in ["authored.p8", "authored.p8.png"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        slot::root::ensure(root);
        std::fs::copy(
            &library,
            root.join("System")
                .join(slot::core::dylib_name(Core::Fake08)),
        )
        .unwrap();
        let cart = root.join("Games/PICO8").join(name);
        if name.ends_with(".png") {
            cart_png(&cart, text.split_once("__lua__\n").unwrap().1);
        } else {
            std::fs::write(&cart, text).unwrap();
        }
        slot_store::write_slot_state(
            root,
            &SlotState {
                cart: Some(name.into()),
                cart_platform: Some(Platform::Pico8),
                clock_set: true,
                ..SlotState::default()
            },
        )
        .unwrap();
        let ring = StateRing::new(root, Platform::Pico8, Core::Fake08, name);
        ring.write_resume(b"not a usable fake08 state").unwrap();
        let mut first = Session::boot_with(root.into(), Box::new(StubSink::draining()));
        let before = frame(&mut first, 128);
        assert!(first.app().may_load_state());
        assert!(!first.app().may_rewind());
        first.app_mut().flush_resume();
        drop(first);
        let save_dir = slot::persist::core_save_dir(root, Platform::Pico8, name);
        assert!(save_dir.join("cdata/slot_test.p8d.txt").is_file());
        let mut second = Session::boot_with(root.into(), Box::new(StubSink::draining()));
        let after = frame(&mut second, 128);
        assert_ne!(
            &before[4..8],
            &after[4..8],
            "{name}: saved progress was not read on reopen"
        );
        assert!(ring.read_resume().unwrap().is_none());
        let states = root.join("States/PICO8/fake08").join(name);
        let refused = std::fs::read_dir(states)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("resume-refused-")
            })
            .expect("invalid resume must be retained for recovery, not overwritten");
        assert_eq!(
            std::fs::read(refused).unwrap(),
            b"not a usable fake08 state"
        );
        drop(second);
    }
}

#[test]
#[ignore = "requires SLOT_SWAN_CORE and SLOT_SOFTWARE_FIXTURE_DIR; real Swan gameplay acceptance gate"]
fn swan_authored_disc_must_execute_gameplay_not_only_firmware_frames() {
    use slot_retro::ButtonMask;
    let library = std::env::var_os("SLOT_SWAN_CORE").expect("set SLOT_SWAN_CORE");
    let content = std::path::PathBuf::from(
        std::env::var_os("SLOT_SOFTWARE_FIXTURE_DIR").expect("export authored fixtures"),
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    slot::root::ensure(root);
    std::fs::copy(
        library,
        root.join("System")
            .join(slot::core::dylib_name(Core::SwanStation)),
    )
    .unwrap();
    for file in ["first.cue", "authored.bin"] {
        std::fs::copy(content.join(file), root.join("Games/PS").join(file)).unwrap();
    }
    let rom = root.join("Games/PS/first.cue");
    let mut opened = slot::core::open_core(
        root,
        Core::SwanStation,
        Platform::Ps,
        &rom,
        "",
        false,
        slot_store::Blending::Off,
        None,
        None,
    )
    .unwrap();
    assert!(opened.named);
    opened.core.load(&rom).unwrap();
    assert!(!opened.core.supports_states());
    assert!(opened.core.serialize().is_err());
    let mut red_frames = 0;
    let mut audio_samples = 0;
    for _ in 0..1200 {
        opened.core.run_frame(ButtonMask::default());
        let pixels = opened.core.video_xrgb8888();
        let red = pixels
            .chunks_exact(4)
            .filter(|p| p[2] > 200 && p[0] < 20 && p[1] < 20)
            .count();
        if red > pixels.len() / 8 {
            red_frames += 1;
        }
        audio_samples += opened.core.take_audio().len();
        if red_frames >= 10 {
            break;
        }
    }
    eprintln!("Swan acceptance: geometry={:?}, authored_red_frames={red_frames}, audio_samples={audio_samples}", opened.core.video_geometry());
    assert!(
        red_frames >= 10,
        "Swan did not execute authored disc gameplay; firmware video/audio is not acceptance"
    );
    assert_eq!(
        opened.core.save_ram().unwrap()[8192],
        1,
        "authored game must write its card through emulated BIOS calls"
    );
}

#[test]
#[ignore = "requires SLOT_SWAN_CORE and SLOT_SWAN_ACCEPTANCE_DIR built with build-acceptance.py"]
fn swan_authored_input_audio_disc_reads_and_cards_survive_reopen() {
    use slot_retro::{ButtonMask, Controller, RetroCore};
    fn colour(core: &mut dyn RetroCore, input: u16, bgr: [u8; 3], limit: usize) {
        for frame in 0..limit {
            core.run_frame(ButtonMask(input));
            core.take_audio();
            let pixels = core.video_xrgb8888();
            if !pixels.is_empty()
                // PS1 RGB555 output can lose low bits when exported as RGB565.
                && pixels.chunks_exact(4).filter(|p| {
                    p[..3].iter().zip(bgr).all(|(&actual, expected)| actual.abs_diff(expected) <= 8)
                }).count() * 8 > pixels.len()
            {
                eprintln!("Swan authored BGR {bgr:?} after {frame} frames");
                return;
            }
        }
        panic!(
            "authored BGR {bgr:?} not reached; geometry={:?}",
            core.video_geometry()
        );
    }
    fn audio(core: &mut dyn RetroCore, input: u16) -> (usize, u64) {
        let mut samples = 0;
        let mut energy = 0;
        for frame in 0..120 {
            core.run_frame(ButtonMask(input));
            let data = core.take_audio();
            if frame >= 90 {
                samples += data.len();
                energy += data
                    .iter()
                    .map(|&s| i64::from(s).unsigned_abs())
                    .sum::<u64>();
            }
        }
        (samples, energy)
    }
    let content = std::path::PathBuf::from(
        std::env::var_os("SLOT_SWAN_ACCEPTANCE_DIR").expect("build authored acceptance discs"),
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    slot::root::ensure(root);
    std::fs::copy(
        std::env::var_os("SLOT_SWAN_CORE").unwrap(),
        root.join("System")
            .join(slot::core::dylib_name(Core::SwanStation)),
    )
    .unwrap();
    for file in [
        "first.cue",
        "first.bin",
        "second.cue",
        "second.bin",
        "authored.m3u",
    ] {
        std::fs::copy(content.join(file), root.join("Games/PS").join(file)).unwrap();
    }
    let name = "authored.m3u";
    let rom = root.join("Games/PS").join(name);
    slot::persist::write_sav_for(
        root,
        Platform::Ps,
        Core::PcsxRearmed,
        name,
        b"PCSX must stay untouched",
    )
    .unwrap();
    for counter in 1..=3 {
        let mut opened = slot::core::open_core(
            root,
            Core::SwanStation,
            Platform::Ps,
            &rom,
            "",
            false,
            slot_store::Blending::Off,
            None,
            None,
        )
        .unwrap();
        assert!(opened.named);
        opened.core.set_controller(Controller::DualShock);
        opened.core.load(&rom).unwrap();
        if let Some(saved) =
            slot::persist::read_sav_for(root, Platform::Ps, Core::SwanStation, name).unwrap()
        {
            opened.core.load_save_ram(&saved).unwrap();
        }
        colour(opened.core.as_mut(), 0, [0, 0, 255], 1800);
        assert_eq!(
            &opened.core.save_ram().unwrap()[8192..8194],
            &[counter, b'S']
        );
        assert!(!opened.core.supports_states());
        assert!(opened.core.serialize().is_err());
        assert!(opened.core.unserialize(b"not a Swan state").is_err());
        if counter == 1 {
            let idle = audio(opened.core.as_mut(), 0);
            colour(opened.core.as_mut(), ButtonMask::RIGHT, [0, 255, 0], 120);
            let sounding = audio(opened.core.as_mut(), ButtonMask::RIGHT);
            colour(opened.core.as_mut(), 0, [0, 0, 255], 120);
            let released = audio(opened.core.as_mut(), 0);
            eprintln!("Swan authored audio (samples, absolute energy): idle={idle:?}, held={sounding:?}, released={released:?}");
            assert!(idle.0 > 10000 && sounding.0 > 10000 && released.0 > 10000);
            assert!(sounding.1 > 100000, "game noise must actually sound");
            assert!(
                idle.1 * 100 < sounding.1 && released.1 * 100 < sounding.1,
                "game input must gate audio, not just firmware callbacks"
            );
            assert_eq!(opened.core.disc_count(), 2);
            colour(opened.core.as_mut(), ButtonMask::B, [0, 255, 255], 900);
            colour(opened.core.as_mut(), 0, [0, 0, 255], 120);
            opened.core.select_disc(1).unwrap();
            assert_eq!(opened.core.selected_disc(), 1);
            audio(opened.core.as_mut(), 0);
            colour(opened.core.as_mut(), ButtonMask::B, [255, 0, 0], 900);
            colour(opened.core.as_mut(), 0, [0, 0, 255], 120);
            opened.core.select_disc(0).unwrap();
            audio(opened.core.as_mut(), 0);
            colour(opened.core.as_mut(), ButtonMask::B, [0, 255, 255], 900);
            assert!(opened.core.select_disc(2).is_err());
            assert_eq!(opened.core.selected_disc(), 0);
        }
        let saved = opened.core.save_ram().unwrap();
        slot::persist::flush(
            root,
            Platform::Ps,
            Core::SwanStation,
            name,
            None,
            Some(&saved),
        )
        .unwrap();
        drop(opened);
        assert_eq!(
            slot::persist::read_sav_for(root, Platform::Ps, Core::SwanStation, name)
                .unwrap()
                .unwrap(),
            saved
        );
        assert_eq!(
            slot::persist::read_sav(root, Platform::Ps, name)
                .unwrap()
                .unwrap(),
            b"PCSX must stay untouched"
        );
        assert!(StateRing::new(root, Platform::Ps, Core::SwanStation, name)
            .read_resume()
            .unwrap()
            .is_none());
    }
    // Exercise Slot's worker, exit and restart, not only the direct core loader.
    std::fs::write(root.join("Games/PS/other.m3u"), "first.cue\n").unwrap();
    slot_store::write_selected_core(root, name, Core::SwanStation).unwrap();
    for counter in 4..=5 {
        slot_store::write_slot_state(
            root,
            &SlotState {
                cart: Some(name.into()),
                cart_platform: Some(Platform::Ps),
                clock_set: true,
                ps_dualshock: true,
                ..SlotState::default()
            },
        )
        .unwrap();
        let mut session = Session::boot_with(root.into(), Box::new(StubSink::draining()));
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            session.update(0.02);
            if session.frame().is_some_and(|f| {
                f.geometry.width == 320
                    && f.chunks_exact(4)
                        .filter(|p| p[2] > 240 && p[0] < 8 && p[1] < 8)
                        .count()
                        * 8
                        > f.len()
            }) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Slot did not reach authored gameplay"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!session.app().may_load_state());
        assert!(!session.app().may_rewind());
        for row in [
            slot_ui::QuickRow::AutoLoad,
            slot_ui::QuickRow::EjectSave,
            slot_ui::QuickRow::Rewind,
        ] {
            assert_eq!(
                session.app().quick_value(row),
                Some(slot_ui::QuickValue::Unavailable)
            );
        }
        for action in [slot_input::Action::SaveState, slot_input::Action::LoadState] {
            session.app_mut().apply(action);
        }
        session.app_mut().flush_resume();
        session.app_mut().apply(slot_input::Action::Eject);
        while !matches!(session.app().phase(), slot::app::Phase::Shelf) {
            session.update(0.02);
            assert!(Instant::now() < deadline, "Slot did not exit Swan gameplay");
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(session);
        let saved = slot::persist::read_sav_for(root, Platform::Ps, Core::SwanStation, name)
            .unwrap()
            .unwrap();
        assert_eq!(&saved[8192..8194], &[counter, b'S']);
        let ring = StateRing::new(root, Platform::Ps, Core::SwanStation, name);
        assert!(!ring.has_resume());
        assert!(ring.list().unwrap().is_empty());
        assert_eq!(
            slot::persist::read_sav(root, Platform::Ps, name)
                .unwrap()
                .unwrap(),
            b"PCSX must stay untouched"
        );
        eprintln!("Swan real Slot exit/restart persisted authored counter {counter}");
    }
}

#[test]
#[ignore = "requires SLOT_PS1_CORE and SLOT_SOFTWARE_FIXTURE_DIR with authored CUE/CHD/PBP fixtures"]
fn ps1_formats_save_and_reopen_through_real_sessions() {
    let library = std::env::var_os("SLOT_PS1_CORE").expect("set SLOT_PS1_CORE");
    let content = std::path::PathBuf::from(
        std::env::var_os("SLOT_SOFTWARE_FIXTURE_DIR").expect("export authored fixtures"),
    );
    for name in ["first.cue", "authored.m3u", "authored.chd", "authored.pbp"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        slot::root::ensure(root);
        std::fs::copy(
            &library,
            root.join("System")
                .join(slot::core::dylib_name(Core::PcsxRearmed)),
        )
        .unwrap();
        for file in [
            "first.cue",
            "second.cue",
            "authored.bin",
            "authored.m3u",
            "authored.chd",
            "authored.pbp",
        ] {
            if file == name
                || (name.ends_with(".cue") && file == "authored.bin")
                || (name.ends_with(".m3u")
                    && matches!(file, "first.cue" | "second.cue" | "authored.bin"))
            {
                std::fs::copy(content.join(file), root.join("Games/PS").join(file)).unwrap();
            }
        }
        slot_store::write_slot_state(
            root,
            &SlotState {
                cart: Some(name.into()),
                cart_platform: Some(Platform::Ps),
                clock_set: true,
                auto_load: false,
                ps_dualshock: true,
                ..SlotState::default()
            },
        )
        .unwrap();
        let mut first = Session::boot_with(root.into(), Box::new(StubSink::draining()));
        let pixels = frame(&mut first, 320);
        assert!(
            pixels.chunks_exact(4).any(|p| p[2] > 200 && p[0] < 20),
            "{name}: authored red frame missing"
        );
        first.app_mut().flush_resume();
        drop(first);
        let saved = slot::persist::read_sav(root, Platform::Ps, name)
            .unwrap()
            .unwrap();
        assert_eq!(saved[8192], 1, "{name}: first BIOS card write missing");
        let mut second = Session::boot_with(root.into(), Box::new(StubSink::draining()));
        frame(&mut second, 320);
        second.app_mut().flush_resume();
        drop(second);
        assert_eq!(
            slot::persist::read_sav(root, Platform::Ps, name)
                .unwrap()
                .unwrap()[8192],
            2,
            "{name}: game did not read and increment the persisted save"
        );
    }
}

fn cart_png(path: &Path, lua: &str) {
    let mut bytes = vec![0; 160 * 205];
    bytes[0x4300..0x4300 + lua.len()].copy_from_slice(lua.as_bytes());
    bytes[0x8000] = 42;
    let rgba: Vec<u8> = bytes
        .into_iter()
        .flat_map(|b| {
            [
                0xfc | ((b >> 4) & 3),
                0xfc | ((b >> 2) & 3),
                0xfc | (b & 3),
                0xfc | (b >> 6),
            ]
        })
        .collect();
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 160, 205);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&rgba)
        .unwrap();
}
