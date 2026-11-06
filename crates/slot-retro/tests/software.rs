use slot_retro::{ButtonMask, Controller, LibretroCore, RetroCore};
use std::path::{Path, PathBuf};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("slot-software-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/software.c");
        let output = dir.join(format!("fixture.{}", std::env::consts::DLL_EXTENSION));
        let status = std::process::Command::new("cc")
            .args([
                if cfg!(target_os = "macos") {
                    "-dynamiclib"
                } else {
                    "-shared"
                },
                "-fPIC",
            ])
            .arg(source)
            .arg("-o")
            .arg(output)
            .status()
            .expect("contract tests require a C compiler");
        assert!(status.success(), "compile libretro contract fixture");
        Self(dir)
    }

    fn core(&self) -> LibretroCore {
        LibretroCore::open_with(
            &self
                .0
                .join(format!("fixture.{}", std::env::consts::DLL_EXTENSION)),
            &self.0,
            &self.0,
        )
        .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires SLOT_FAKE08_CORE built with cores/ps1-pico8/build.sh"]
fn fake08_cart_progress_survives_a_new_core_session() {
    unsafe extern "C" {
        fn fcntl(fd: i32, cmd: i32) -> i32;
    }
    let dir = std::env::temp_dir().join(format!("slot-fake08-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let scratch = Fixture(dir);
    let path = PathBuf::from(std::env::var_os("SLOT_FAKE08_CORE").expect("set SLOT_FAKE08_CORE"));
    let cart = scratch.0.join("authored.p8");
    std::fs::write(&cart, include_str!("fixtures/persistence.p8")).unwrap();
    let open = || {
        let mut c = LibretroCore::open_with(&path, &scratch.0, &scratch.0).unwrap();
        c.set_native_video();
        c.disable_states();
        c.load(&cart).unwrap();
        c
    };
    let mut c = open();
    for _ in 0..10 {
        c.run_frame(ButtonMask::default());
    }
    assert_eq!(
        (c.video_geometry().width, c.video_geometry().height),
        (128, 128)
    );
    assert_eq!(c.video_xrgb8888().len(), 128 * 128 * 4);
    let first = c.video_xrgb8888().to_vec();
    assert!(
        first[4..].chunks_exact(4).all(|p| p == &first[4..8]),
        "cart painted a uniform saved-progress colour"
    );
    assert!(
        first[4..7].iter().any(|&v| v != 0),
        "cart ran, not a black uninitialised screen"
    );
    c.run_frame(ButtonMask(ButtonMask::B));
    assert_ne!(
        &c.video_xrgb8888()[128 * 128 * 4 - 4..],
        &first[4..8],
        "button changes bottom-right pixel"
    );
    assert!(
        c.take_audio().iter().any(|&s| s != 0),
        "authored tone is audible, not just silence callbacks"
    );
    assert!(!c.supports_states());
    assert!(matches!(
        c.serialize(),
        Err(slot_retro::CoreError::Unsupported(_))
    ));
    assert!(matches!(
        c.unserialize(&[1, 2, 3, 4]),
        Err(slot_retro::CoreError::Unsupported(_))
    ));
    drop(c);
    assert_ne!(
        unsafe { fcntl(2, 1) },
        -1,
        "core must not close the host diagnostic stream"
    );
    assert!(scratch.0.join("cdata/slot_test.p8d.txt").is_file());
    let mut fresh = open();
    for _ in 0..10 {
        fresh.run_frame(ButtonMask::default());
    }
    assert_ne!(
        &fresh.video_xrgb8888()[4..8],
        &first[4..8],
        "fresh _init increments previously saved progress"
    );
}

#[test]
#[ignore = "future fake08 serialization repair gate; requires SLOT_FAKE08_CORE"]
fn fake08_quick_state_restores_lua_closures_and_shared_tables_after_reopen() {
    let dir = std::env::temp_dir().join(format!("slot-fake08-state-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let scratch = Fixture(dir);
    let path = PathBuf::from(std::env::var_os("SLOT_FAKE08_CORE").expect("set SLOT_FAKE08_CORE"));
    let cart = scratch.0.join("state.p8");
    std::fs::write(&cart, include_str!("fixtures/quick-state.p8")).unwrap();
    let open = || {
        let mut c = LibretroCore::open_with(&path, &scratch.0, &scratch.0).unwrap();
        c.set_native_video();
        // This isolated contract probe deliberately bypasses the production capability gate.
        c.load(&cart).unwrap();
        c
    };
    let mut c = open();
    for _ in 0..4 {
        c.run_frame(ButtonMask::default());
    }
    let initial = c.video_xrgb8888().to_vec();
    // Cover both update phases of a 30 Hz cart presented at 60 Hz.
    for _ in 0..2 {
        c.run_frame(ButtonMask(ButtonMask::LEFT));
    }
    for _ in 0..2 {
        c.run_frame(ButtonMask::default());
    }
    let expected = c.video_xrgb8888().to_vec();
    assert!(expected != initial, "authored Lua gameplay must advance");
    let state = c
        .serialize()
        .expect("a quick state must serialize Lua state");
    for _ in 0..2 {
        c.run_frame(ButtonMask(ButtonMask::LEFT));
    }
    for _ in 0..2 {
        c.run_frame(ButtonMask::default());
    }
    assert!(c.video_xrgb8888() != expected);
    c.unserialize(&state).unwrap();
    c.run_frame(ButtonMask::default());
    assert!(
        c.video_xrgb8888() == expected,
        "Lua counter/table/coroutine/globals must roll back: got {:?}, expected {:?}",
        &c.video_xrgb8888()[..24],
        &expected[..24]
    );
    drop(c);
    let mut reopened = open();
    for _ in 0..4 {
        reopened.run_frame(ButtonMask::default());
    }
    assert!(reopened.video_xrgb8888() == initial);
    reopened.unserialize(&state).unwrap();
    reopened.run_frame(ButtonMask::default());
    assert!(
        reopened.video_xrgb8888() == expected,
        "fresh Lua VM must resume the saved closures and shared table"
    );
}

#[test]
#[ignore = "future fake08 serialization repair gate; requires SLOT_FAKE08_CORE"]
fn fake08_state_continuation_preserves_input_time_random_and_audio_and_rejects_damage() {
    let dir = std::env::temp_dir().join(format!("slot-fake08-timing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let scratch = Fixture(dir);
    let path = PathBuf::from(std::env::var_os("SLOT_FAKE08_CORE").expect("set SLOT_FAKE08_CORE"));
    let cart = scratch.0.join("timing.p8");
    std::fs::write(&cart, include_str!("fixtures/state-timing.p8")).unwrap();
    let open = || {
        let mut c = LibretroCore::open_with(&path, &scratch.0, &scratch.0).unwrap();
        c.set_native_video();
        c.load(&cart).unwrap();
        c
    };
    let continuation = |c: &mut LibretroCore| {
        (0..36)
            .map(|n| {
                c.run_frame(ButtonMask(if n < 23 { ButtonMask::LEFT } else { 0 }));
                (c.video_xrgb8888().to_vec(), c.take_audio())
            })
            .collect::<Vec<_>>()
    };
    let mut c = open();
    for _ in 0..45 {
        c.run_frame(ButtonMask(ButtonMask::LEFT));
        c.take_audio();
    }
    let state = c
        .serialize()
        .expect("snapshot a held button at odd audio parity");
    let expected = continuation(&mut c);
    assert!(
        expected
            .iter()
            .any(|(_, audio)| audio.iter().any(|&v| v != 0)),
        "fixture must play noise audio"
    );
    assert!(
        expected.windows(2).any(|w| w[0].0 != w[1].0),
        "fixture must expose changing gameplay/time/random state"
    );
    c.unserialize(&state).unwrap();
    assert!(
        continuation(&mut c) == expected,
        "input repeat, time, PRNG, audio samples and audio parity must roll back"
    );
    c.unserialize(&state).unwrap();
    for offset in [0, 8, 80, 100, state.len() / 2, state.len() - 1] {
        let mut damaged = state.clone();
        damaged[offset] ^= 0x5a;
        assert!(
            c.unserialize(&damaged).is_err(),
            "corrupt state accepted at {offset}"
        );
    }
    assert!(c.unserialize(&state[..state.len() - 1]).is_err());
    let mut oversized = state.clone();
    oversized.push(0);
    assert!(c.unserialize(&oversized).is_err());
    // Valid outer checksum, invalid inner Lua stream: exercise staged parsing,
    // rather than stopping at the inexpensive corruption check.
    let mut invalid_lua = state.clone();
    let runtime_size = u32::from_le_bytes(state[84..88].try_into().unwrap()) as usize;
    let lua_size = u32::from_le_bytes(state[88..92].try_into().unwrap()) as usize;
    invalid_lua[144 + runtime_size..144 + runtime_size + lua_size].fill(0);
    let checksum = invalid_lua
        .iter()
        .enumerate()
        .filter(|(i, _)| !(136..140).contains(i))
        .fold(2166136261u32, |hash, (_, &byte)| {
            (hash ^ u32::from(byte)).wrapping_mul(16777619)
        });
    invalid_lua[136..140].copy_from_slice(&checksum.to_le_bytes());
    assert!(c.unserialize(&invalid_lua).is_err());
    assert!(
        continuation(&mut c) == expected,
        "failed restores must not change live gameplay or audio"
    );
    drop(c);
    let mut reopened = open();
    reopened.unserialize(&state).unwrap();
    assert!(
        continuation(&mut reopened) == expected,
        "fresh core must restore audio, input, PRNG and VM timing"
    );
}

#[test]
#[ignore = "requires patched SLOT_FAKE08_CORE"]
fn fake08_refuses_oversized_or_callback_snapshots_without_losing_gameplay() {
    let path = PathBuf::from(std::env::var_os("SLOT_FAKE08_CORE").expect("set SLOT_FAKE08_CORE"));
    for unsupported in [
        "(function() local s='x' for i=1,21 do s=s..s end return s end)()",
        "setmetatable({},{__persist=function() error('must not execute') end})",
        "setmetatable({},{__gc=function() error('must not execute') end})",
    ] {
        let dir = std::env::temp_dir().join(format!("slot-fake08-bounds-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let scratch = Fixture(dir);
        let cart = scratch.0.join("bounds.p8");
        std::fs::write(&cart, format!(
            "pico-8 cartridge // authored snapshot error fixture\nversion 42\n__lua__\nlocal n=1\nfunction _update60()\n if btnp(0) then n+=1 end\n if btnp(1) then payload={unsupported} end\nend\nfunction _draw() cls(n%16) end\n"
        )).unwrap();
        let mut c = LibretroCore::open_with(&path, &scratch.0, &scratch.0).unwrap();
        c.set_native_video();
        c.load(&cart).unwrap();
        c.run_frame(ButtonMask::default());
        let good = c.serialize().unwrap();
        let initial = c.video_xrgb8888().to_vec();
        c.run_frame(ButtonMask(ButtonMask::RIGHT));
        assert!(
            c.serialize().is_err(),
            "unsupported snapshot accepted: {unsupported}"
        );
        c.run_frame(ButtonMask(ButtonMask::LEFT));
        assert!(
            c.video_xrgb8888() != initial,
            "save failure stopped gameplay"
        );
        c.unserialize(&good).unwrap();
        c.run_frame(ButtonMask::default());
        assert!(
            c.video_xrgb8888() == initial,
            "prior good save must remain usable"
        );
    }
}

#[test]
#[ignore = "requires SLOT_PS1_CORE built with cores/ps1-pico8/build.sh"]
fn pcsx_hle_executes_authored_mips_and_exposes_a_memory_card() {
    let dir = std::env::temp_dir().join(format!("slot-pcsx-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let scratch = Fixture(dir);
    let path = PathBuf::from(std::env::var_os("SLOT_PS1_CORE").expect("set SLOT_PS1_CORE"));
    let exe = scratch.0.join("authored.exe");
    let mut data = vec![0; 4096];
    data[..8].copy_from_slice(b"PS-X EXE");
    for (offset, value) in [
        (0x10, 0x80010000u32),
        (0x18, 0x80010000),
        (0x1c, 2048),
        (0x30, 0x801fff00),
    ] {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    // Initialise the BIOS card driver before file access. PCSX HLE also works
    // without StartCARD/_bu_init, but a firmware BIOS needs the full sequence.
    // File IO is executed by the emulated CPU, not injected through the save API.
    // A saved 'S' marker distinguishes our counter from unspecified new-file
    // bytes (PCSX HLE zeroes them; OpenBIOS preserves erased card bytes).
    let mut code: Vec<u32> = vec![
        0x00002021, 0x2409004a, 0x0c00002c, 0, 0x2409004b, 0x0c00002c, 0, 0x24090070, 0x0c000028,
        0, 0x3c048001, 0x34840500, 0x3c050001, 0x34a50203, 0x24090032, 0x0c00002c, 0, 0x00408021,
        0x02002021, 0x3c058001, 0x34a50600, 0x24060080, 0x24090034, 0x0c00002c, 0, 0x3c088001,
        0x35080600, 0x910a0000, 0x910b0001, 0x240c0053, 0x116c0002, 0, 0x00005021, 0xa10c0001,
        0x254a0001, 0xa10a0000, 0x02002021, 0x00002821, 0x00003021, 0x24090033, 0x0c00002c, 0,
        0x02002021, 0x3c058001, 0x34a50600, 0x24060080, 0x24090035, 0x0c00002c, 0, 0x02002021,
        0x24090036, 0x0c00002c, 0,
    ];
    // Enable 320×240 output and paint the VRAM red using GP1/GP0 registers.
    code.extend_from_slice(&[
        0x3c081f80, 0x3c090300, 0xad091814, 0x3c090800, 0x35290001, 0xad091814, 0x3c090500,
        0xad091814, 0x3c0906c6, 0x35290260, 0xad091814, 0x3c090704, 0x35292018, 0xad091814,
        0x3c090200, 0x352900ff, 0xad091810, 0xad001810, 0x3c0900f0, 0x35290140, 0xad091810, 0, 0,
        0, 0, 0, 0, 0x1000ffff, 0,
    ]);
    for (n, word) in code.iter().enumerate() {
        data[2048 + n * 4..2052 + n * 4].copy_from_slice(&word.to_le_bytes());
    }
    data[2048 + 0x500..2048 + 0x500 + 15].copy_from_slice(b"bu00:SLOT_TEST\0");
    std::fs::write(&exe, &data).unwrap();
    // An authored ISO9660 disc exercises the supported CUE/M3U path, rather
    // than counting a standalone EXE load as evidence for disc formats.
    let mut iso = vec![0; 64 * 2048];
    iso[16 * 2048..16 * 2048 + 7].copy_from_slice(b"\x01CD001\x01");
    iso[16 * 2048 + 40..16 * 2048 + 72].fill(b' ');
    iso[16 * 2048 + 80..16 * 2048 + 84].copy_from_slice(&64u32.to_le_bytes());
    iso[16 * 2048 + 84..16 * 2048 + 88].copy_from_slice(&64u32.to_be_bytes());
    iso[16 * 2048 + 120..16 * 2048 + 128].copy_from_slice(&[1, 0, 0, 1, 1, 0, 0, 1]);
    iso[16 * 2048 + 128..16 * 2048 + 132].copy_from_slice(&[0, 8, 8, 0]);
    // OpenBIOS follows the ISO9660 path table; PCSX HLE tolerates its absence.
    iso[16 * 2048 + 132..16 * 2048 + 136].copy_from_slice(&10u32.to_le_bytes());
    iso[16 * 2048 + 136..16 * 2048 + 140].copy_from_slice(&10u32.to_be_bytes());
    iso[16 * 2048 + 140..16 * 2048 + 144].copy_from_slice(&18u32.to_le_bytes());
    iso[16 * 2048 + 148..16 * 2048 + 152].copy_from_slice(&19u32.to_be_bytes());
    iso[18 * 2048..18 * 2048 + 10].copy_from_slice(&[1, 0, 20, 0, 0, 0, 1, 0, 0, 0]);
    iso[19 * 2048..19 * 2048 + 10].copy_from_slice(&[1, 0, 0, 0, 0, 20, 0, 1, 0, 0]);
    let root = iso_record(&[0], 20, 2048, true);
    iso[16 * 2048 + 156..16 * 2048 + 156 + root.len()].copy_from_slice(&root);
    iso[17 * 2048..17 * 2048 + 7].copy_from_slice(b"\xffCD001\x01");
    let config = b"BOOT = cdrom:\\AUTHORED.EXE;1\r\nTCB = 4\r\nEVENT = 10\r\nSTACK = 801fff00\r\n";
    let mut offset = 20 * 2048;
    for record in [
        root,
        iso_record(&[1], 20, 2048, true),
        iso_record(b"SYSTEM.CNF;1", 21, config.len() as u32, false),
        iso_record(b"AUTHORED.EXE;1", 22, data.len() as u32, false),
    ] {
        iso[offset..offset + record.len()].copy_from_slice(&record);
        offset += record.len();
    }
    iso[21 * 2048..21 * 2048 + config.len()].copy_from_slice(config);
    iso[22 * 2048..22 * 2048 + data.len()].copy_from_slice(&data);
    let mut tracks = vec![0; 64 * 2352];
    for (n, sector) in iso.chunks_exact(2048).enumerate() {
        let raw = &mut tracks[n * 2352..(n + 1) * 2352];
        raw[1..11].fill(0xff);
        let bcd = |value: usize| ((value / 10) * 16 + value % 10) as u8;
        raw[12..16].copy_from_slice(&[
            bcd((n + 150) / 4500),
            bcd((n + 150) / 75 % 60),
            bcd(n % 75),
            2,
        ]);
        raw[18] = 8;
        raw[22] = 8;
        raw[24..24 + 2048].copy_from_slice(sector);
    }
    std::fs::write(scratch.0.join("authored.bin"), tracks).unwrap();
    for name in ["first.cue", "second.cue"] {
        std::fs::write(
            scratch.0.join(name),
            "FILE \"authored.bin\" BINARY\n TRACK 01 MODE2/2352\n INDEX 01 00:00:00\n",
        )
        .unwrap();
    }
    let playlist = scratch.0.join("authored.m3u");
    std::fs::write(&playlist, "first.cue\nsecond.cue\n").unwrap();
    if let Some(out) = std::env::var_os("SLOT_SOFTWARE_FIXTURE_DIR") {
        let out = PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        for name in ["authored.bin", "first.cue", "second.cue", "authored.m3u"] {
            std::fs::copy(scratch.0.join(name), out.join(name)).unwrap();
        }
    }
    let mut c = LibretroCore::open_preset(
        &path,
        &scratch.0,
        &scratch.0,
        &[
            ("pcsx_rearmed_bios", "HLE"),
            ("pcsx_rearmed_memcard1", "libretro"),
            ("pcsx_rearmed_neon_enhancement_enable", "disabled"),
        ],
    )
    .unwrap();
    c.set_native_video();
    c.set_controller(Controller::DualShock);
    c.load(&playlist).unwrap();
    c.set_analog([[-12345, 23456], [32767, -32768]]);
    for _ in 0..60 {
        c.run_frame(ButtonMask::default());
    }
    let g = c.video_geometry();
    assert_eq!(g.width, 320);
    assert!((224..=240).contains(&g.height));
    assert_eq!(c.video_xrgb8888().len(), (g.width * g.height * 4) as usize);
    assert!(
        c.video_xrgb8888()
            .chunks_exact(4)
            .any(|p| p[2] > 200 && p[0] < 20 && p[1] < 20),
        "authored MIPS code painted red, not just a successful load"
    );
    let card = c.save_ram().expect("libretro-managed first memory card");
    assert_eq!(card.len(), 128 * 1024);
    assert_eq!(card[8192], 1, "authored game saved through BIOS card calls");
    let state = c.serialize().unwrap();
    assert!(state.len() > 1024 * 1024);
    assert!(c.unserialize(&state[..state.len() - 1]).is_err());
    let mut oversized = state.clone();
    oversized.push(0);
    assert!(c.unserialize(&oversized).is_err());
    let mut corrupt = state.clone();
    corrupt[..9].fill(0);
    assert!(c.unserialize(&corrupt).is_err());
    // Corrupt the redundant SPU length, forcing a stream seek out of range.
    let spu = state
        .windows(6)
        .position(|bytes| bytes == b"PBOSS\0")
        .unwrap();
    corrupt.copy_from_slice(&state);
    corrupt[spu - 4..spu].copy_from_slice(&i32::MAX.to_le_bytes());
    assert!(c.unserialize(&corrupt).is_err());
    c.run_frame(ButtonMask::default());
    c.unserialize(&state).unwrap();
    assert_eq!(c.disc_count(), 2);
    c.select_disc(1).unwrap();
    assert_eq!(c.selected_disc(), 1);
    let second_disc_state = c.serialize().unwrap();
    c.select_disc(0).unwrap();
    c.unserialize(&second_disc_state).unwrap();
    assert_eq!(
        c.selected_disc(),
        1,
        "PCSX states restore the selected disc"
    );
    drop(c);
    let mut fresh = LibretroCore::open_preset(
        &path,
        &scratch.0,
        &scratch.0,
        &[
            ("pcsx_rearmed_bios", "HLE"),
            ("pcsx_rearmed_memcard1", "libretro"),
        ],
    )
    .unwrap();
    fresh.set_native_video();
    fresh.load(&playlist).unwrap();
    fresh.load_save_ram(&card).unwrap();
    for _ in 0..60 {
        fresh.run_frame(ButtonMask::default());
    }
    assert_eq!(
        fresh.save_ram().unwrap()[8192],
        2,
        "fresh game read and incremented its prior card save"
    );
}

fn iso_record(name: &[u8], sector: u32, size: u32, directory: bool) -> Vec<u8> {
    let len = (33 + name.len() + 1) & !1;
    let mut record = vec![0; len];
    record[0] = len as u8;
    record[2..6].copy_from_slice(&sector.to_le_bytes());
    record[6..10].copy_from_slice(&sector.to_be_bytes());
    record[10..14].copy_from_slice(&size.to_le_bytes());
    record[14..18].copy_from_slice(&size.to_be_bytes());
    record[25] = if directory { 2 } else { 0 };
    record[28..32].copy_from_slice(&[1, 0, 0, 1]);
    record[32] = name.len() as u8;
    record[33..33 + name.len()].copy_from_slice(name);
    record
}

#[test]
fn fullpath_content_is_not_read_into_memory() {
    let fixture = Fixture::new();
    let rom = fixture.0.join("disc.cue");
    std::fs::write(&rom, b"FILE disc.bin BINARY\n").unwrap();
    let mut core = fixture.core();
    assert!(core.uses_gb_buttons());
    assert!(core.sync_to_display());
    core.set_native_input();
    assert!(!core.uses_gb_buttons());
    core.set_native_video();
    assert!(!core.sync_to_display());
    core.set_controller(Controller::DualShock);
    core.load(&rom)
        .expect("a full-path core must receive null data and zero size");
    core.set_analog([[i16::MIN, 1234], [23456, -30000]]);
    core.run_frame(ButtonMask(
        ButtonMask::X
            | ButtonMask::Y
            | ButtonMask::L2
            | ButtonMask::R2
            | ButtonMask::L3
            | ButtonMask::R3,
    ));
    let g = core.video_geometry();
    assert_eq!((g.width, g.height), (320, 224));
    assert!((g.aspect - 4.0 / 3.0).abs() < 0.001);
    assert_eq!(core.video_xrgb8888().len(), 320 * 224 * 4);
    let pixel =
        |n: usize| u32::from_ne_bytes(core.video_xrgb8888()[n * 4..n * 4 + 4].try_into().unwrap());
    assert_eq!(pixel(0), 0x112233);
    assert_eq!(
        pixel(320 * 224 - 1),
        0x445566,
        "bottom right survived padded pitch and no crop"
    );
    assert_eq!(pixel(1), 517, "PCSX DualShock subclass must survive load");
    assert_eq!(
        [pixel(2), pixel(3), pixel(4), pixel(5)],
        [32768, 1234, 23456, 35536]
    );
    assert_eq!(
        pixel(6),
        0xf202,
        "all face buttons, triggers and stick clicks are genuine inputs"
    );
    assert_eq!(core.av_info().fps, 50.0);
    core.set_controller(Controller::Digital);
    core.set_analog([[0; 2]; 2]);
    core.run_frame(ButtonMask::default());
    assert_eq!(&core.video_xrgb8888()[4..8], &1u32.to_ne_bytes());
    assert!(core.video_xrgb8888()[8..28].iter().all(|&b| b == 0));
    assert_eq!(core.disc_count(), 3);
    assert_eq!(core.selected_disc(), 0);
    core.select_disc(1).unwrap();
    assert_eq!(core.selected_disc(), 1);
    core.run_frame(ButtonMask::default());
    assert_eq!(
        &core.video_xrgb8888()[28..32],
        &1u32.to_ne_bytes(),
        "disc one is inserted, not left ejected"
    );
    assert!(
        core.select_disc(2).is_err(),
        "fixture refuses a listed but unreadable disc"
    );
    assert!(
        core.select_disc(3).is_err(),
        "out-of-range disc is rejected"
    );
    assert_eq!(core.selected_disc(), 1);
    core.run_frame(ButtonMask::default());
    assert_eq!(
        &core.video_xrgb8888()[28..32],
        &1u32.to_ne_bytes(),
        "refusal leaves the prior disc inserted"
    );
    let kept = core.video_xrgb8888().to_vec();
    for button in [ButtonMask::START, ButtonMask::B, ButtonMask::SELECT] {
        core.run_frame(ButtonMask(button));
        assert!(
            core.video_xrgb8888() == kept,
            "duplicate, invalid pitch and oversized frames preserve the last valid picture"
        );
    }
    core.run_frame(ButtonMask(ButtonMask::A));
    assert_eq!(
        (core.video_geometry().width, core.video_geometry().height),
        (320, 448)
    );
    assert_eq!(
        &core.video_xrgb8888()[320 * 448 * 4 - 4..],
        &0x778899u32.to_ne_bytes()
    );
    assert_eq!(
        (core.av_info().fps, core.av_info().sample_rate),
        (60.0, 48000.0)
    );
    core.run_frame(ButtonMask(ButtonMask::L));
    assert_eq!(
        (core.video_geometry().width, core.video_geometry().height),
        (128, 128)
    );
    assert_eq!(core.video_geometry().aspect, 1.0);
    assert_eq!(core.video_xrgb8888().len(), 128 * 128 * 4);
    assert_eq!(&core.video_xrgb8888()[..4], &[0, 0, 255, 0]);
    assert_eq!(&core.video_xrgb8888()[128 * 128 * 4 - 4..], &[0, 255, 0, 0]);
    assert!(
        core.video_xrgb8888()[4..128 * 128 * 4 - 4]
            .iter()
            .all(|&b| b == 0),
        "shrinking has no old frame borders or row padding"
    );
    let advertised = fixture.0.join("advertised.cue");
    std::fs::write(&advertised, b"FILE disc.bin BINARY\n").unwrap();
    core.set_controller(Controller::DualShock);
    core.load(&advertised).unwrap();
    core.run_frame(ButtonMask::default());
    assert_eq!(
        &core.video_xrgb8888()[4..8],
        &261u32.to_ne_bytes(),
        "DualShock must use the advertised core ID, not the PCSX-specific ID"
    );
    core.set_controller(Controller::Digital);
    core.run_frame(ButtonMask::default());
    assert_eq!(&core.video_xrgb8888()[4..8], &1u32.to_ne_bytes());
}
