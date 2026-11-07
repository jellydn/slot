//! Opt-in offscreen diagnostic. Never part of the production gameplay path.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {
    eprintln!("swan-hw-probe requires macOS desktop GL or Linux EGL/GLES");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
#[path = "swan-hw-probe/egl.rs"]
mod egl;

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    unsafe {
        extern "C" {
            fn alarm(seconds: u32) -> u32;
        }
        // Bound even a blocked vendor/core call. Process teardown owns timeout cleanup.
        alarm(300);
    }
    probe::run()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod probe {
    #[cfg(target_os = "linux")]
    use crate::egl::Pbuffer as HeadlessSurface;
    use libloading::Library;
    #[cfg(target_os = "macos")]
    use slot_gfx::{HeadlessSurface, Surface};
    use slot_retro::host_probe::*;
    use slot_retro::ButtonMask;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::ffi::{c_char, c_uint, c_void, CStr, CString};
    use std::io::Write;
    use std::path::Path;
    use std::ptr;
    use std::time::{Duration, Instant};

    const MAX_W: u32 = 1024;
    const MAX_H: u32 = 512;

    // Exact libretro C layout. Contexts are accepted only for this diagnostic.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Hardware {
        kind: c_uint,
        reset: Option<unsafe extern "C" fn()>,
        framebuffer: Option<unsafe extern "C" fn() -> usize>,
        proc: Option<unsafe extern "C" fn(*const c_char) -> *const c_void>,
        depth: bool,
        stencil: bool,
        bottom_left: bool,
        major: c_uint,
        minor: c_uint,
        cache: bool,
        destroy: Option<unsafe extern "C" fn()>,
        debug: bool,
    }

    struct Target {
        fbo: u32,
        texture: u32,
        width: u32,
        height: u32,
    }

    impl Target {
        fn new(width: u32, height: u32) -> Result<Self, String> {
            if width == 0 || height == 0 || width > MAX_W || height > MAX_H {
                return Err("offscreen dimensions outside prototype bounds".into());
            }
            let mut target = Self {
                fbo: 0,
                texture: 0,
                width,
                height,
            };
            unsafe {
                gl::GenTextures(1, &mut target.texture);
                gl::BindTexture(gl::TEXTURE_2D, target.texture);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA8 as i32,
                    width as i32,
                    height as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    ptr::null(),
                );
                gl::GenFramebuffers(1, &mut target.fbo);
                gl::BindFramebuffer(gl::FRAMEBUFFER, target.fbo);
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    target.texture,
                    0,
                );
                let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
                if status != gl::FRAMEBUFFER_COMPLETE {
                    return Err(format!("offscreen framebuffer incomplete: {status:#x}"));
                }
            }
            Ok(target)
        }

        fn read(&self, width: u32, height: u32, bottom_left: bool) -> Result<Vec<u8>, String> {
            if width == 0 || height == 0 || width > self.width || height > self.height {
                return Err("readback outside owned framebuffer".into());
            }
            let mut pixels = vec![0; (width * height * 4) as usize];
            unsafe {
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.fbo);
                gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
                gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
                gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
                gl::PixelStorei(gl::PACK_ROW_LENGTH, 0);
                gl::PixelStorei(gl::PACK_SKIP_PIXELS, 0);
                gl::PixelStorei(gl::PACK_SKIP_ROWS, 0);
                gl::ReadPixels(
                    0,
                    0,
                    width as i32,
                    height as i32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    pixels.as_mut_ptr().cast(),
                );
                let error = gl::GetError();
                if error != gl::NO_ERROR {
                    return Err(format!("GL readback error {error:#x}"));
                }
            }
            if bottom_left {
                let row = width as usize * 4;
                for y in 0..height as usize / 2 {
                    let other = height as usize - y - 1;
                    for x in 0..row {
                        pixels.swap(y * row + x, other * row + x);
                    }
                }
            }
            for pixel in pixels.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
            Ok(pixels)
        }
    }

    impl Drop for Target {
        fn drop(&mut self) {
            unsafe {
                gl::DeleteFramebuffers(1, &self.fbo);
                gl::DeleteTextures(1, &self.texture);
            }
        }
    }

    struct Host {
        // Single-threaded ownership: targets are dropped before the current context.
        target: Target,
        surface: HeadlessSurface,
        hw: Option<Hardware>,
        allow_hw: bool,
        options: HashMap<String, CString>,
        system: CString,
        save: CString,
        disk: Option<DiskControl>,
        format: u32,
        input: u16,
        pixels: Vec<u8>,
        size: (u32, u32),
        audio: Vec<i16>,
        fresh: u64,
        hardware_frames: u64,
        readback: Duration,
        error: Option<String>,
    }

    thread_local! { static ACTIVE: Cell<*mut Host> = const { Cell::new(ptr::null_mut()) }; }

    // No callback may keep a Host borrow across a call back into the core.
    unsafe fn host<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
        ACTIVE.with(|slot| slot.get().as_mut().map(f))
    }

    unsafe extern "C" fn framebuffer() -> usize {
        host(|h| h.target.fbo as usize).unwrap_or(0)
    }

    unsafe extern "C" fn proc(name: *const c_char) -> *const c_void {
        if name.is_null() {
            return ptr::null();
        }
        let Ok(name) = CStr::from_ptr(name).to_str() else {
            return ptr::null();
        };
        host(|h| h.surface.proc_address(name)).unwrap_or(ptr::null())
    }

    unsafe extern "C" fn environment(cmd: u32, data: *mut c_void) -> bool {
        if data.is_null() {
            return false;
        }
        match cmd {
            GET_CAN_DUPE => {
                *data.cast::<bool>() = true;
                true
            }
            GET_INPUT_BITMASKS => true,
            SET_PIXEL_FORMAT => {
                let format = *data.cast::<u32>();
                if format != PIXEL_FORMAT_XRGB8888 && format != PIXEL_FORMAT_RGB565 {
                    return false;
                }
                host(|h| h.format = format).is_some()
            }
            GET_SYSTEM_DIRECTORY | GET_SAVE_DIRECTORY => host(|h| {
                *data.cast::<*const c_char>() = if cmd == GET_SYSTEM_DIRECTORY {
                    h.system.as_ptr()
                } else {
                    h.save.as_ptr()
                };
                true
            })
            .unwrap_or(false),
            GET_VARIABLE => {
                let var = &mut *data.cast::<Variable>();
                if var.key.is_null() {
                    return false;
                }
                let Ok(key) = CStr::from_ptr(var.key).to_str() else {
                    return false;
                };
                host(|h| {
                    var.value = h.options.get(key).map_or(ptr::null(), |v| v.as_ptr());
                    !var.value.is_null()
                })
                .unwrap_or(false)
            }
            GET_VARIABLE_UPDATE => {
                *data.cast::<bool>() = false;
                true
            }
            SET_VARIABLES | SET_CONTROLLER_INFO | SET_GEOMETRY | SET_SYSTEM_AV_INFO => true,
            SET_DISK_CONTROL_INTERFACE => {
                host(|h| h.disk = Some(*data.cast::<DiskControl>())).is_some()
            }
            56 => {
                *data.cast::<u32>() = if cfg!(target_os = "linux") { 5 } else { 3 };
                true
            }
            14 => {
                let hw = &mut *data.cast::<Hardware>();
                host(|h| {
                    #[cfg(target_os = "macos")]
                    let compatible = hw.kind == 3 && hw.major == 3 && (2..=3).contains(&hw.minor);
                    #[cfg(target_os = "linux")]
                    let compatible = hw.kind == 5 && hw.major == 3 && hw.minor <= 2;
                    if !h.allow_hw || !compatible || hw.depth || hw.stencil || h.hw.is_some() {
                        return false;
                    }
                    hw.framebuffer = Some(framebuffer);
                    hw.proc = Some(proc);
                    h.hw = Some(*hw);
                    true
                })
                .unwrap_or(false)
            }
            _ => false,
        }
    }

    unsafe extern "C" fn video(data: *const c_void, width: u32, height: u32, pitch: usize) {
        if data.is_null() {
            return;
        } // Keep the previous image for duplicate frames.
        host(|h| {
            if width == 0 || height == 0 || width > MAX_W || height > MAX_H {
                h.error = Some("core frame outside prototype bounds".into());
                return;
            }
            let pixels = if data as usize == usize::MAX {
                let Some(hw) = h.hw else {
                    h.error = Some("unnegotiated hardware frame".into());
                    return;
                };
                let began = Instant::now();
                let pixels = h.target.read(width, height, hw.bottom_left);
                h.readback += began.elapsed();
                match pixels {
                    Ok(p) => {
                        h.hardware_frames += 1;
                        p
                    }
                    Err(e) => {
                        h.error = Some(e);
                        return;
                    }
                }
            } else {
                let bpp = if h.format == PIXEL_FORMAT_RGB565 {
                    2
                } else {
                    4
                };
                if pitch < width as usize * bpp || pitch.checked_mul(height as usize).is_none() {
                    h.error = Some("invalid software pitch".into());
                    return;
                }
                let mut out = Vec::with_capacity((width * height * 4) as usize);
                for y in 0..height as usize {
                    let row = data.cast::<u8>().add(y * pitch);
                    for x in 0..width as usize {
                        if bpp == 4 {
                            let p = row.add(x * 4);
                            out.extend_from_slice(&[*p.add(2), *p.add(1), *p, 255]);
                        } else {
                            let p = ptr::read_unaligned(row.add(x * 2).cast::<u16>());
                            let (r, g, b) = (
                                ((p >> 11) & 31) as u8,
                                ((p >> 5) & 63) as u8,
                                (p & 31) as u8,
                            );
                            out.extend_from_slice(&[
                                (r << 3) | (r >> 2),
                                (g << 2) | (g >> 4),
                                (b << 3) | (b >> 2),
                                255,
                            ]);
                        }
                    }
                }
                out
            };
            h.pixels = pixels;
            h.size = (width, height);
            h.fresh += 1;
        });
    }

    unsafe extern "C" fn audio(left: i16, right: i16) {
        host(|h| h.audio.extend_from_slice(&[left, right]));
    }
    unsafe extern "C" fn audio_batch(data: *const i16, frames: usize) -> usize {
        if !data.is_null() && frames <= 192000 {
            host(|h| {
                h.audio
                    .extend_from_slice(std::slice::from_raw_parts(data, frames * 2))
            });
        }
        frames
    }
    unsafe extern "C" fn poll() {}
    unsafe extern "C" fn input(port: u32, device: u32, _index: u32, id: u32) -> i16 {
        if port != 0 || device != DEVICE_JOYPAD {
            return 0;
        }
        host(|h| match id {
            JOYPAD_MASK => h.input as i16,
            0..16 => ((h.input >> id) & 1) as i16,
            _ => 0,
        })
        .unwrap_or(0)
    }

    struct Core {
        api: Api,
        host: Box<Host>,
        loaded: bool,
        _library: Library,
        // Empty BIOS/cards/cache are disposable, never a user root.
        _root: tempfile::TempDir,
    }

    impl Core {
        fn open(library: &Path, hardware: bool) -> Result<Self, Box<dyn std::error::Error>> {
            if ACTIVE.with(|a| !a.get().is_null()) {
                return Err("another probe core is active".into());
            }
            let root = tempfile::tempdir()?;
            std::fs::create_dir(root.path().join("bios"))?;
            std::fs::create_dir(root.path().join("save"))?;
            let surface = HeadlessSurface::new()?;
            gl::load_with(|name| surface.proc_address(name));
            let target = Target::new(MAX_W, MAX_H)?;
            let library = unsafe { Library::new(library)? };
            let api = unsafe { Api::load(&library)? };
            if unsafe { (api.api_version)() } != 1 {
                return Err("unexpected libretro API".into());
            }
            let options = [
                (
                    "swanstation_GPU_Renderer",
                    if hardware { "OpenGL" } else { "Software" },
                ),
                ("swanstation_GPU_ResolutionScale", "1"),
                ("swanstation_GPU_PGXPEnable", "false"),
                ("swanstation_MemoryCards_Card1Type", "Libretro"),
                ("swanstation_MemoryCards_Card2Type", "None"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), CString::new(v).unwrap()))
            .collect();
            let mut core = Self {
                api,
                host: Box::new(Host {
                    target,
                    surface,
                    hw: None,
                    allow_hw: hardware,
                    options,
                    system: CString::new(root.path().join("bios").as_os_str().as_encoded_bytes())?,
                    save: CString::new(root.path().join("save").as_os_str().as_encoded_bytes())?,
                    disk: None,
                    format: PIXEL_FORMAT_XRGB8888,
                    input: 0,
                    pixels: Vec::new(),
                    size: (0, 0),
                    audio: Vec::new(),
                    fresh: 0,
                    hardware_frames: 0,
                    readback: Duration::ZERO,
                    error: None,
                }),
                loaded: false,
                _library: library,
                _root: root,
            };
            ACTIVE.with(|a| a.set(&mut *core.host));
            unsafe {
                (core.api.set_environment)(environment);
                (core.api.set_video_refresh)(video);
                (core.api.set_audio_sample)(audio);
                (core.api.set_audio_sample_batch)(audio_batch);
                (core.api.set_input_poll)(poll);
                (core.api.set_input_state)(input);
                (core.api.init)();
            }
            Ok(core)
        }

        fn load(&mut self, disc: &Path) -> Result<(), Box<dyn std::error::Error>> {
            let path = CString::new(disc.as_os_str().as_encoded_bytes())?;
            let info = GameInfo {
                path: path.as_ptr(),
                data: ptr::null(),
                size: 0,
                meta: ptr::null(),
            };
            if !unsafe { (self.api.load_game)(&info) } {
                return Err("Swan refused authored disc".into());
            }
            self.loaded = true;
            if self.host.allow_hw {
                let hw = self
                    .host
                    .hw
                    .ok_or("Swan did not negotiate hardware rendering")?;
                self.host.surface.make_current()?;
                unsafe {
                    hw.reset.ok_or("missing reset callback")?();
                }
            }
            unsafe {
                (self.api.set_controller_port_device)(0, DEVICE_JOYPAD);
            }
            Ok(())
        }

        fn frames(&mut self, count: usize, mask: u16) -> Result<Duration, String> {
            self.host.input = mask;
            self.host.audio.clear();
            let start = Instant::now();
            for _ in 0..count {
                unsafe {
                    (self.api.run)();
                }
                if let Some(error) = self.host.error.take() {
                    return Err(error);
                }
            }
            Ok(start.elapsed())
        }

        fn card(&mut self) -> Result<&mut [u8], String> {
            unsafe {
                let size = (self.api.get_memory_size)(MEMORY_SAVE_RAM);
                let data = (self.api.get_memory_data)(MEMORY_SAVE_RAM);
                if size != 128 * 1024 || data.is_null() {
                    return Err("expected one libretro memory card".into());
                }
                Ok(std::slice::from_raw_parts_mut(data.cast(), size))
            }
        }
    }

    impl Drop for Core {
        fn drop(&mut self) {
            // The core's callbacks and GL resources must die before the FBO/context/library.
            let _ = self.host.surface.make_current();
            unsafe {
                // Swan's destroy callback recreates its software GPU; the game must
                // still exist here. Unloading first leaves that callback a null GPU.
                if self.loaded {
                    if let Some(destroy) = self.host.hw.take().and_then(|h| h.destroy) {
                        destroy();
                    }
                    (self.api.unload_game)();
                }
                (self.api.deinit)();
            }
            ACTIVE.with(|a| a.set(ptr::null_mut()));
        }
    }

    fn save_image(
        path: &Path,
        size: (u32, u32),
        pixels: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut enc = png::Encoder::new(std::fs::File::create_new(path)?, size.0, size.1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(pixels)?;
        Ok(())
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args: Vec<_> = std::env::args_os().collect();
        if args.len() != 4 {
            return Err("usage: swan-hw-probe CORE_DYLIB AUTHORED_M3U NEW_OUTPUT_DIR".into());
        }
        let output = Path::new(&args[3]);
        std::fs::create_dir(output)?;
        #[cfg(target_os = "linux")]
        {
            // Keep core-created temporary roots inside the caller's disposable output.
            std::env::set_var("TMPDIR", output.canonicalize()?);
        }
        let mut evidence = Vec::new();
        let mut reference = Vec::new();
        let mut previous_card = None;
        for hardware in [false, true, true] {
            let mode = if hardware { "OpenGL" } else { "Software" };
            let mut core = Core::open(Path::new(&args[1]), hardware)?;
            core.load(Path::new(&args[2]))?;
            if let Some(path) = &previous_card {
                let saved = std::fs::read(path)?;
                if saved.len() != 128 * 1024 {
                    return Err("invalid authored card size".into());
                }
                core.card()?.copy_from_slice(&saved);
            }
            core.frames(900, 0)?;
            let counter = evidence.len() as u8 + 1;
            if core.card()?[8192..8194] != [counter, b'S'] {
                return Err("authored card counter did not survive reopen".into());
            }
            let gl_identity: Vec<_> = [gl::VENDOR, gl::RENDERER, gl::VERSION]
                .into_iter()
                .map(|name| unsafe {
                    CStr::from_ptr(gl::GetString(name).cast())
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            let mut colours = Vec::new();
            let mut timings = Vec::new();
            for (stage, mask, disc, expected) in [
                ("idle", 0, None, [255, 0, 0]),
                ("right", ButtonMask::RIGHT, None, [0, 255, 0]),
                ("released", 0, None, [255, 0, 0]),
                ("disc-0", ButtonMask::B, Some(0), [255, 255, 0]),
                ("disc-1", ButtonMask::B, Some(1), [0, 0, 255]),
                ("disc-0-return", ButtonMask::B, Some(0), [255, 255, 0]),
            ] {
                if let Some(index) = disc {
                    core.frames(30, 0)?;
                    let disk = core.host.disk.ok_or("missing disc control")?;
                    unsafe {
                        if disk.get_num_images.ok_or("missing disc count")?() != 2
                            || !disk.set_eject_state.ok_or("missing eject")?(true)
                            || !disk.set_image_index.ok_or("missing disc selection")?(index)
                            || !disk.set_eject_state.ok_or("missing insert")?(false)
                        {
                            return Err("authored disc switch failed".into());
                        }
                    }
                    core.frames(120, 0)?;
                }
                core.frames(60, mask)?;
                let before = core.host.readback;
                let elapsed = core.frames(180, mask)?;
                let pixels = &core.host.pixels;
                if pixels.is_empty() {
                    return Err(format!("{mode} produced no pixels").into());
                }
                let mid = pixels.len() / 8 * 4;
                let colour = &pixels[mid..mid + 3];
                // RGB555→RGB565 software conversion loses one green quantisation step.
                if !colour
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 4)
                {
                    return Err(
                        format!("{mode} {stage}: expected {expected:?}, got {colour:?}").into(),
                    );
                }
                let audible = core.host.audio.iter().any(|&s| s != 0);
                if audible != (mask & ButtonMask::RIGHT != 0) {
                    return Err(
                        format!("{mode} {stage}: unexpected audio activity {audible}").into(),
                    );
                }
                colours.push((core.host.size, pixels.clone()));
                let mut stage_result = serde_json::json!({"stage":stage,"frames":180,"audio_samples":core.host.audio.len(),
                    "audio_abs_energy":core.host.audio.iter().map(|&s| i64::from(s).unsigned_abs()).sum::<u64>()});
                // Linux's bounded compatibility gate is not a performance benchmark.
                if cfg!(target_os = "macos") {
                    stage_result["wall_ms"] = (elapsed.as_secs_f64() * 1000.0).into();
                    stage_result["readback_ms"] =
                        ((core.host.readback - before).as_secs_f64() * 1000.0).into();
                }
                timings.push(stage_result);
                save_image(
                    &output.join(format!("{}-{mode}-{stage}.png", evidence.len())),
                    core.host.size,
                    pixels,
                )?;
            }
            if !hardware {
                reference = colours.clone();
            } else if core.host.hardware_frames == 0
                || core.host.hardware_frames != core.host.fresh
                || colours
                    .iter()
                    .zip(&reference)
                    .any(|((size, pixels), (ref_size, ref_pixels))| {
                        size != ref_size
                            || pixels.len() != ref_pixels.len()
                            || pixels
                                .iter()
                                .zip(ref_pixels)
                                .any(|(&a, &b)| a.abs_diff(b) > 4)
                    })
            {
                return Err("hardware output did not match software or silently fell back".into());
            }
            evidence.push(serde_json::json!({"mode":mode,"size":core.host.size,"hardware_frames":core.host.hardware_frames,
                "fresh_frames":core.host.fresh,"gl_vendor_renderer_version":gl_identity,"timings":timings,"authored_card_counter":counter}));
            // Only this diagnostic's new card is persisted. No user save root is accepted.
            let card = core.card()?.to_vec();
            let path = output.join(format!("authored-card-{counter}.sav"));
            let mut file = std::fs::File::create_new(&path)?;
            file.write_all(&card)?;
            file.sync_all()?;
            drop(file);
            drop(core);
            if std::fs::read(&path)? != card {
                return Err("authored card readback mismatch".into());
            }
            previous_card = Some(path);
        }
        let scope = if cfg!(target_os = "macos") {
            "Mac desktop OpenGL only; not stock GLES or TSP speed"
        } else {
            "Linux offscreen GLES compatibility only; not presentation or speed acceptance"
        };
        let report = serde_json::json!({"scope":scope, "runs":evidence});
        std::fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        println!("{report}");
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn real_gl_readback_preserves_asymmetric_corners_resize_and_bounds() {
            let surface = HeadlessSurface::new().unwrap();
            gl::load_with(|name| surface.proc_address(name));
            for (w, h) in [(7, 5), (320, 224), (320, 448), (3, 9)] {
                let target = Target::new(w, h).unwrap();
                unsafe {
                    gl::Enable(gl::SCISSOR_TEST);
                    for (x, y, colour) in [
                        (0, 0, [1.0, 0.0, 0.0]),
                        (w - 1, 0, [0.0, 1.0, 0.0]),
                        (0, h - 1, [0.0, 0.0, 1.0]),
                        (w - 1, h - 1, [1.0, 1.0, 0.0]),
                    ] {
                        gl::Scissor(x as i32, y as i32, 1, 1);
                        gl::ClearColor(colour[0], colour[1], colour[2], 1.0);
                        gl::Clear(gl::COLOR_BUFFER_BIT);
                    }
                    gl::Disable(gl::SCISSOR_TEST);
                    // A core can leave padded pixel-pack state behind.
                    gl::PixelStorei(gl::PACK_ROW_LENGTH, w as i32 + 7);
                    gl::PixelStorei(gl::PACK_SKIP_ROWS, 2);
                    gl::PixelStorei(gl::PACK_SKIP_PIXELS, 3);
                }
                let pixels = target.read(w, h, true).unwrap();
                let pixel = |x: u32, y: u32| {
                    &pixels[((y * w + x) * 4) as usize..((y * w + x) * 4 + 4) as usize]
                };
                assert_eq!(pixel(0, 0), &[0, 0, 255, 255]);
                assert_eq!(pixel(w - 1, 0), &[255, 255, 0, 255]);
                assert_eq!(pixel(0, h - 1), &[255, 0, 0, 255]);
                assert_eq!(pixel(w - 1, h - 1), &[0, 255, 0, 255]);
                let unflipped = target.read(w, h, false).unwrap();
                assert_eq!(&unflipped[..4], &[255, 0, 0, 255]);
                assert!(target.read(w + 1, h, true).is_err());
                assert!(target.read(w, 0, true).is_err());
            }
            assert!(Target::new(MAX_W + 1, 1).is_err());
            assert!(Target::new(1, MAX_H + 1).is_err());
        }
    }
}
