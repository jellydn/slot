// SPDX-License-Identifier: GPL-3.0-or-later
//! Standalone, opt-in Linux host diagnostic. No Slot, Swan, window or device I/O.
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Linux surfaceless EGL host required; no platform fallback");
    std::process::exit(2);
}

#[cfg(target_os = "linux")]
fn main() {
    unsafe { diagnostic::alarm(180) };
    let result = diagnostic::run();
    if diagnostic::live_resources() || diagnostic::cleanup_failed() {
        eprintln!("STOP reason=cleanup-failure");
        std::process::exit(2);
    }
    if let Err(reason) = result {
        eprintln!("STOP reason={reason}");
        std::process::exit(2);
    }
}

#[cfg(target_os = "linux")]
mod diagnostic {
    use std::ffi::CStr;
    use std::marker::PhantomData;
    use std::os::raw::{c_char, c_void};
    use std::ptr;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    type Result<T> = std::result::Result<T, &'static str>;
    type Handle = *mut c_void;
    const NONE: i32 = 0x3038;
    const WIDTH: i32 = 1024;
    const HEIGHT: i32 = 512;
    const FRAMEBUFFER: u32 = 0x8d40;
    const TEXTURE: u32 = 0x0de1;
    const COLOUR: u32 = 0x4000;
    const SCISSOR: u32 = 0x0c11;
    const PACK_ALIGNMENT: u32 = 0x0d05;
    static CONTEXTS: AtomicUsize = AtomicUsize::new(0);
    static SURFACES: AtomicUsize = AtomicUsize::new(0);
    static DISPLAYS: AtomicUsize = AtomicUsize::new(0);
    static SYNCS: AtomicUsize = AtomicUsize::new(0);
    static CLEANUP_FAILED: AtomicBool = AtomicBool::new(false);

    #[link(name = "EGL")]
    extern "C" {
        fn eglGetDisplay(native: Handle) -> Handle;
        fn eglInitialize(display: Handle, major: *mut i32, minor: *mut i32) -> u32;
        fn eglTerminate(display: Handle) -> u32;
        fn eglBindAPI(api: u32) -> u32;
        fn eglChooseConfig(
            display: Handle,
            attrs: *const i32,
            config: *mut Handle,
            count: i32,
            found: *mut i32,
        ) -> u32;
        fn eglCreatePbufferSurface(display: Handle, config: Handle, attrs: *const i32) -> Handle;
        fn eglCreateContext(
            display: Handle,
            config: Handle,
            shared: Handle,
            attrs: *const i32,
        ) -> Handle;
        fn eglDestroyContext(display: Handle, context: Handle) -> u32;
        fn eglDestroySurface(display: Handle, surface: Handle) -> u32;
        fn eglMakeCurrent(display: Handle, draw: Handle, read: Handle, context: Handle) -> u32;
        fn eglQueryString(display: Handle, name: i32) -> *const c_char;
        fn eglGetProcAddress(name: *const c_char) -> *const c_void;
    }
    extern "C" {
        pub fn alarm(seconds: u32) -> u32;
    }

    macro_rules! require {
        ($condition:expr, $reason:literal) => {
            if !$condition {
                return Err($reason);
            }
        };
    }
    macro_rules! api {
        ($($field:ident: $name:literal ($($arg:ty),*) -> $ret:ty;)+) => {
            #[derive(Clone, Copy)]
            struct Gl { $($field: unsafe extern "C" fn($($arg),*) -> $ret,)+ }
            impl Gl {
                fn load() -> Result<Self> {
                    unsafe {
                        Ok(Self { $($field: {
                            let p = eglGetProcAddress(concat!($name, "\0").as_ptr().cast());
                            require!(!p.is_null(), "missing-gl-entry-point");
                            std::mem::transmute::<*const c_void, unsafe extern "C" fn($($arg),*) -> $ret>(p)
                        },)+ })
                    }
                }
            }
        };
    }
    api! {
        get_error: "glGetError" () -> u32;
        get_string: "glGetString" (u32) -> *const c_char;
        gen_textures: "glGenTextures" (i32, *mut u32) -> ();
        delete_textures: "glDeleteTextures" (i32, *const u32) -> ();
        bind_texture: "glBindTexture" (u32, u32) -> ();
        tex_parameter: "glTexParameteri" (u32, u32, i32) -> ();
        tex_image: "glTexImage2D" (u32, i32, i32, i32, i32, i32, u32, u32, *const c_void) -> ();
        gen_fbos: "glGenFramebuffers" (i32, *mut u32) -> ();
        delete_fbos: "glDeleteFramebuffers" (i32, *const u32) -> ();
        bind_fbo: "glBindFramebuffer" (u32, u32) -> ();
        attach: "glFramebufferTexture2D" (u32, u32, u32, u32, i32) -> ();
        fbo_status: "glCheckFramebufferStatus" (u32) -> u32;
        enable: "glEnable" (u32) -> ();
        disable: "glDisable" (u32) -> ();
        scissor: "glScissor" (i32, i32, i32, i32) -> ();
        clear_colour: "glClearColor" (f32, f32, f32, f32) -> ();
        clear: "glClear" (u32) -> ();
        blit: "glBlitFramebuffer" (i32, i32, i32, i32, i32, i32, i32, i32, u32, u32) -> ();
        flush: "glFlush" () -> ();
        get_integer: "glGetIntegerv" (u32, *mut i32) -> ();
        pack: "glPixelStorei" (u32, i32) -> ();
        read: "glReadPixels" (i32, i32, i32, i32, u32, u32, *mut c_void) -> ();
    }
    type CreateSync = unsafe extern "C" fn(Handle, u32, *const i32) -> Handle;
    type WaitSync = unsafe extern "C" fn(Handle, Handle, i32, u64) -> i32;
    type DestroySync = unsafe extern "C" fn(Handle, Handle) -> u32;
    type SignalSync = unsafe extern "C" fn(Handle, Handle, u32) -> u32;

    // EGLDisplay and EGLSync handles are cross-thread EGL objects. Contexts and
    // GL containers below are !Send/!Sync. Numeric tokens are never made current.
    struct Display {
        handle: usize,
        config: usize,
        create: CreateSync,
        wait: WaitSync,
        destroy: DestroySync,
        signal: SignalSync,
        created: AtomicUsize,
        destroyed: AtomicUsize,
    }
    impl Display {
        fn new() -> Result<Arc<Self>> {
            unsafe {
                macro_rules! load {
                    ($name:literal, $ty:ty) => {{
                        let p = eglGetProcAddress(concat!($name, "\0").as_ptr().cast());
                        require!(!p.is_null(), "missing-egl-entry-point");
                        std::mem::transmute::<*const c_void, $ty>(p)
                    }};
                }
                // Resolve fallible entry points before acquiring a display.
                let mut owner = Self {
                    handle: 0,
                    config: 0,
                    create: load!("eglCreateSyncKHR", CreateSync),
                    wait: load!("eglClientWaitSyncKHR", WaitSync),
                    destroy: load!("eglDestroySyncKHR", DestroySync),
                    signal: load!("eglSignalSyncKHR", SignalSync),
                    created: AtomicUsize::new(0),
                    destroyed: AtomicUsize::new(0),
                };
                let handle = eglGetDisplay(ptr::null_mut());
                require!(
                    !handle.is_null()
                        && eglInitialize(handle, ptr::null_mut(), ptr::null_mut()) != 0,
                    "egl-initialize"
                );
                owner.handle = handle as usize;
                DISPLAYS.fetch_add(1, Ordering::SeqCst);
                require!(eglBindAPI(0x30a0) != 0, "egl-bind-api");
                let extensions = eglQueryString(handle, 0x3055);
                require!(!extensions.is_null(), "egl-extensions");
                let extensions = CStr::from_ptr(extensions).to_bytes();
                for required in [
                    b"EGL_KHR_fence_sync".as_slice(),
                    b"EGL_KHR_reusable_sync".as_slice(),
                ] {
                    require!(
                        extensions.split(|&b| b == b' ').any(|s| s == required),
                        "missing-sync-extension"
                    );
                }
                let attrs = [
                    0x3033, 1, 0x3040, 0x44, 0x3024, 8, 0x3023, 8, 0x3022, 8, 0x3021, 8, NONE,
                ];
                let mut config = ptr::null_mut();
                let mut count = 0;
                require!(
                    eglChooseConfig(handle, attrs.as_ptr(), &mut config, 1, &mut count) != 0
                        && count == 1
                        && !config.is_null(),
                    "egl-pbuffer-config"
                );
                owner.config = config as usize;
                Ok(Arc::new(owner))
            }
        }
        fn poll(&self, handle: usize) -> Result<bool> {
            require!(handle != 0, "missing-fence");
            match unsafe { (self.wait)(self.handle as Handle, handle as Handle, 0, 0) } {
                0x30f6 => Ok(true),
                0x30f5 => Ok(false),
                _ => Err("fence-wait-failed"),
            }
        }
        fn signal(&self, handle: usize) -> Result<()> {
            require!(
                unsafe { (self.signal)(self.handle as Handle, handle as Handle, 0x30f2) } != 0,
                "signal-failed"
            );
            Ok(())
        }
    }
    impl Drop for Display {
        fn drop(&mut self) {
            if self.handle != 0 {
                if unsafe { eglTerminate(self.handle as Handle) } == 0 {
                    CLEANUP_FAILED.store(true, Ordering::SeqCst);
                }
                DISPLAYS.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }
    struct Context {
        display: Arc<Display>,
        surface: usize,
        handle: usize,
        gl: Option<Gl>,
        _thread_local: PhantomData<Rc<()>>,
    }
    impl Context {
        fn new(display: Arc<Display>, shared: usize, version: i32, fail: &str) -> Result<Self> {
            let mut context = Self {
                display,
                surface: 0,
                handle: 0,
                gl: None,
                _thread_local: PhantomData,
            };
            unsafe {
                require!(eglBindAPI(0x30a0) != 0, "egl-bind-api");
                let size = [0x3057, 64, 0x3056, 64, NONE];
                context.surface = eglCreatePbufferSurface(
                    context.display.handle as Handle,
                    context.display.config as Handle,
                    size.as_ptr(),
                ) as usize;
                require!(context.surface != 0, "egl-pbuffer");
                SURFACES.fetch_add(1, Ordering::SeqCst);
                require!(fail != "surface", "startup-surface");
                let attrs = [0x3098, version, NONE];
                context.handle = eglCreateContext(
                    context.display.handle as Handle,
                    context.display.config as Handle,
                    shared as Handle,
                    attrs.as_ptr(),
                ) as usize;
                require!(context.handle != 0, "egl-context");
                CONTEXTS.fetch_add(1, Ordering::SeqCst);
                require!(fail != "context", "startup-context");
                require!(
                    eglMakeCurrent(
                        context.display.handle as Handle,
                        context.surface as Handle,
                        context.surface as Handle,
                        context.handle as Handle
                    ) != 0,
                    "egl-current"
                );
                context.gl = Some(Gl::load()?);
                require!(fail != "current", "startup-current");
            }
            Ok(context)
        }
        fn gl(&self) -> Gl {
            self.gl.expect("current context has local dispatch")
        }
        fn identity(&self, label: &str) -> Result<()> {
            unsafe {
                let version = (self.gl().get_string)(0x1f02);
                let renderer = (self.gl().get_string)(0x1f01);
                require!(!version.is_null() && !renderer.is_null(), "gl-identity");
                println!(
                    "{label}_actual={} renderer={}",
                    CStr::from_ptr(version).to_string_lossy(),
                    CStr::from_ptr(renderer).to_string_lossy()
                );
            }
            Ok(())
        }
    }
    impl Drop for Context {
        fn drop(&mut self) {
            unsafe {
                if self.handle != 0 {
                    let detached = eglMakeCurrent(
                        self.display.handle as Handle,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        ptr::null_mut(),
                    );
                    let destroyed =
                        eglDestroyContext(self.display.handle as Handle, self.handle as Handle);
                    if detached == 0 || destroyed == 0 {
                        CLEANUP_FAILED.store(true, Ordering::SeqCst);
                    }
                    CONTEXTS.fetch_sub(1, Ordering::SeqCst);
                }
                if self.surface != 0 {
                    if eglDestroySurface(self.display.handle as Handle, self.surface as Handle) == 0
                    {
                        CLEANUP_FAILED.store(true, Ordering::SeqCst);
                    }
                    SURFACES.fetch_sub(1, Ordering::SeqCst);
                }
            }
        }
    }
    struct Fence {
        handle: usize,
        display: Arc<Display>,
    }
    impl Fence {
        fn new(display: &Arc<Display>, gl: Gl, reusable: bool) -> Result<Self> {
            let handle = unsafe {
                (display.create)(
                    display.handle as Handle,
                    if reusable { 0x30fa } else { 0x30f9 },
                    ptr::null(),
                )
            } as usize;
            require!(handle != 0, "fence-create-failed");
            display.created.fetch_add(1, Ordering::SeqCst);
            SYNCS.fetch_add(1, Ordering::SeqCst);
            if !reusable {
                unsafe { (gl.flush)() };
            }
            Ok(Self {
                handle,
                display: display.clone(),
            })
        }
    }
    impl Drop for Fence {
        fn drop(&mut self) {
            if unsafe {
                (self.display.destroy)(self.display.handle as Handle, self.handle as Handle)
            } == 0
            {
                CLEANUP_FAILED.store(true, Ordering::SeqCst);
            }
            self.display.destroyed.fetch_add(1, Ordering::SeqCst);
            SYNCS.fetch_sub(1, Ordering::SeqCst);
        }
    }
    struct Target {
        texture: u32,
        fbo: u32,
        gl: Gl,
        lost: Arc<AtomicBool>,
        _thread_local: PhantomData<Rc<()>>,
    }
    impl Target {
        fn new(gl: Gl, lost: &Arc<AtomicBool>, allocate_texture: bool) -> Result<Self> {
            let mut target = Self {
                texture: 0,
                fbo: 0,
                gl,
                lost: lost.clone(),
                _thread_local: PhantomData,
            };
            unsafe {
                if allocate_texture {
                    (gl.gen_textures)(1, &mut target.texture);
                    (gl.bind_texture)(TEXTURE, target.texture);
                    for (name, value) in [
                        (0x2801, 0x2600),
                        (0x2800, 0x2600),
                        (0x2802, 0x812f),
                        (0x2803, 0x812f),
                    ] {
                        (gl.tex_parameter)(TEXTURE, name, value);
                    }
                    (gl.tex_image)(
                        TEXTURE,
                        0,
                        0x8058,
                        WIDTH,
                        HEIGHT,
                        0,
                        0x1908,
                        0x1401,
                        ptr::null(),
                    );
                }
                (gl.gen_fbos)(1, &mut target.fbo);
                require!(
                    target.fbo != 0 && (!allocate_texture || target.texture != 0),
                    "gl-resource-create"
                );
                if allocate_texture {
                    (gl.bind_fbo)(FRAMEBUFFER, target.fbo);
                    (gl.attach)(FRAMEBUFFER, 0x8ce0, TEXTURE, target.texture, 0);
                    require!((gl.fbo_status)(FRAMEBUFFER) == 0x8cd5, "incomplete-target");
                }
                require!((gl.get_error)() == 0, "gl-target-error");
            }
            Ok(target)
        }
    }
    impl Drop for Target {
        fn drop(&mut self) {
            if !self.lost.load(Ordering::SeqCst) {
                unsafe {
                    (self.gl.delete_fbos)(1, &self.fbo);
                    (self.gl.delete_textures)(1, &self.texture);
                    if (self.gl.get_error)() != 0 {
                        CLEANUP_FAILED.store(true, Ordering::SeqCst);
                    }
                }
            }
        }
    }

    #[derive(Clone, Copy, Default)]
    struct Frame {
        sequence: u64,
        generation: u64,
        width: i32,
        height: i32,
        bottom_left: bool,
    }
    impl Frame {
        fn valid(self) -> bool {
            self.width > 0 && self.height > 0 && self.width <= WIDTH && self.height <= HEIGHT
        }
    }
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Free,
        Writing,
        Ready,
        Reading,
        Retiring,
    }
    struct Slot {
        state: State,
        texture: u32,
        frame: Frame,
        fence: Option<Fence>,
    }
    struct Pool {
        slots: [Slot; 3],
        generation: u64,
    }
    impl Pool {
        fn new(generation: u64) -> Self {
            Self {
                generation,
                slots: std::array::from_fn(|_| Slot {
                    state: State::Free,
                    texture: 0,
                    frame: Frame::default(),
                    fence: None,
                }),
            }
        }
    }
    type SharedPool = Arc<Mutex<Pool>>;
    enum Command {
        Render(Frame),
        Stop,
    }
    fn tick() {
        thread::sleep(Duration::from_millis(1));
    }
    fn wait(display: &Display, handle: usize) -> Result<()> {
        for _ in 0..250 {
            if display.poll(handle)? {
                return Ok(());
            }
            tick();
        }
        Err("fence-timeout")
    }
    fn recycle(pool: &SharedPool, display: &Display) -> Result<()> {
        for i in 0..3 {
            let handle = {
                let pool = pool.lock().unwrap();
                let slot = &pool.slots[i];
                if slot.state != State::Retiring {
                    continue;
                }
                slot.fence.as_ref().map_or(0, |f| f.handle)
            };
            if display.poll(handle)? {
                let fence = {
                    let mut pool = pool.lock().unwrap();
                    let slot = &mut pool.slots[i];
                    slot.state = State::Free;
                    slot.fence.take()
                };
                drop(fence);
            }
        }
        Ok(())
    }
    fn render(gl: Gl, target: &Target, f: Frame) -> Result<()> {
        require!(f.valid(), "unsupported-geometry");
        unsafe {
            (gl.bind_fbo)(FRAMEBUFFER, target.fbo);
            (gl.enable)(SCISSOR);
            for y in 0..2 {
                for x in 0..2 {
                    let left = if x == 0 { 0 } else { f.width / 2 };
                    let right = if x == 0 { f.width / 2 } else { f.width };
                    let top = if y == 0 { 0 } else { f.height / 2 };
                    let bottom = if y == 0 { f.height / 2 } else { f.height };
                    (gl.scissor)(
                        left,
                        if f.bottom_left {
                            f.height - bottom
                        } else {
                            top
                        },
                        right - left,
                        bottom - top,
                    );
                    (gl.clear_colour)(
                        (f.sequence & 255) as f32 / 255.0,
                        ((if y == 0 { 31 } else { 161 }) + ((f.sequence >> 8) & 63)) as f32 / 255.0,
                        (if x == 0 { 17.0 } else { 223.0 }) / 255.0,
                        1.0,
                    );
                    (gl.clear)(COLOUR);
                }
            }
            (gl.disable)(SCISSOR);
        }
        Ok(())
    }
    fn worker(
        display: Arc<Display>,
        shared: usize,
        pool: SharedPool,
        lost: Arc<AtomicBool>,
        commands: mpsc::Receiver<Command>,
        replies: mpsc::SyncSender<bool>,
        fault: String,
    ) -> Result<()> {
        let context = Context::new(display.clone(), shared, 3, "")?;
        let gl = context.gl();
        if pool.lock().unwrap().generation == 1 {
            context.identity("producer")?;
        }
        let stable = Target::new(gl, &lost, true)?;
        let targets = [
            Target::new(gl, &lost, true)?,
            Target::new(gl, &lost, true)?,
            Target::new(gl, &lost, true)?,
        ];
        // On any error, invalidate before target destructors run. A worker
        // failure must not delete textures still leased by the consumer.
        struct StopOnError {
            lost: Arc<AtomicBool>,
            armed: bool,
        }
        impl Drop for StopOnError {
            fn drop(&mut self) {
                if self.armed {
                    self.lost.store(true, Ordering::SeqCst);
                }
            }
        }
        let mut stop_on_error = StopOnError {
            lost: lost.clone(),
            armed: true,
        };
        {
            let mut pool = pool.lock().unwrap();
            for (slot, target) in pool.slots.iter_mut().zip(&targets) {
                slot.texture = target.texture;
            }
        }
        if replies.send(false).is_err() {
            return Ok(());
        }
        while let Ok(command) = commands.recv() {
            if lost.load(Ordering::SeqCst) {
                break;
            }
            let mut frame = match command {
                Command::Stop => break,
                Command::Render(frame) => frame,
            };
            recycle(&pool, &display)?;
            render(gl, &stable, frame)?;
            let free = {
                let mut pool = pool.lock().unwrap();
                pool.slots
                    .iter_mut()
                    .enumerate()
                    .find(|(_, s)| s.state == State::Free)
                    .map(|(i, s)| {
                        s.state = State::Writing;
                        i
                    })
            };
            if let Some(i) = free {
                unsafe {
                    (gl.bind_fbo)(0x8ca8, stable.fbo);
                    (gl.bind_fbo)(0x8ca9, targets[i].fbo);
                    (gl.blit)(
                        0,
                        0,
                        frame.width,
                        frame.height,
                        0,
                        0,
                        frame.width,
                        frame.height,
                        COLOUR,
                        0x2600,
                    );
                    require!((gl.get_error)() == 0, "gl-blit-error");
                }
                let fence = if fault == "producer-fence" {
                    None
                } else {
                    Some(Fence::new(&display, gl, false)?)
                };
                let fence = if fault == "timeout" {
                    if let Some(ref fence) = fence {
                        wait(&display, fence.handle)?;
                    }
                    drop(fence);
                    Some(Fence::new(&display, gl, true)?)
                } else {
                    fence
                };
                if fault == "generation" {
                    frame.generation -= 1;
                }
                if fault == "origin" {
                    frame.bottom_left = !frame.bottom_left;
                }
                let mut pool = pool.lock().unwrap();
                pool.slots[i].frame = frame;
                pool.slots[i].fence = fence;
                pool.slots[i].state = State::Ready;
            }
            if replies.send(free.is_some()).is_err() {
                return Ok(());
            }
        }
        if !lost.load(Ordering::SeqCst) {
            let mut drained = false;
            for _ in 0..250 {
                recycle(&pool, &display)?;
                drained = pool
                    .lock()
                    .unwrap()
                    .slots
                    .iter()
                    .all(|s| s.state == State::Free);
                if drained {
                    break;
                }
                tick();
            }
            require!(drained, "shutdown-fence-timeout");
        }
        // Targets die before their current context; the display outlives both threads.
        stop_on_error.armed = false;
        drop(targets);
        drop(stable);
        drop(context);
        Ok(())
    }

    // A lease is not released by Drop: only an explicit flushed consumer fence
    // permits reuse. Dropping an unfinished session invalidates the whole pool.
    struct Lease {
        index: usize,
        frame: Frame,
        texture: u32,
    }
    struct Session {
        reader: Option<Target>,
        context: Option<Context>,
        display: Arc<Display>,
        pool: SharedPool,
        lost: Arc<AtomicBool>,
        commands: Option<mpsc::SyncSender<Command>>,
        replies: Option<mpsc::Receiver<bool>>,
        worker: Option<JoinHandle<Result<()>>>,
        lease: Option<Lease>,
        busy: bool,
        sequence: u64,
        acquired: u64,
        fault: String,
        captures: usize,
        publications: usize,
        drops: usize,
        finished: bool,
    }
    impl Session {
        fn new(generation: u64, fault: &str) -> Result<Self> {
            let display = Display::new()?;
            let lost = Arc::new(AtomicBool::new(false));
            let context = Context::new(display.clone(), 0, 2, "")?;
            if generation == 1 {
                context.identity("consumer_requested2")?;
            }
            let reader = Target::new(context.gl(), &lost, false)?;
            unsafe {
                (context.gl().pack)(PACK_ALIGNMENT, 8);
            }
            let pool = Arc::new(Mutex::new(Pool::new(generation)));
            let (commands, receive_commands) = mpsc::sync_channel(1);
            let (send_replies, replies) = mpsc::sync_channel(1);
            let worker_display = display.clone();
            let worker_pool = pool.clone();
            let worker_lost = lost.clone();
            let shared = context.handle;
            let worker_fault = fault.to_owned();
            let worker = thread::spawn(move || {
                worker(
                    worker_display,
                    shared,
                    worker_pool,
                    worker_lost,
                    receive_commands,
                    send_replies,
                    worker_fault,
                )
            });
            let mut session = Self {
                reader: Some(reader),
                context: Some(context),
                display,
                pool,
                lost,
                commands: Some(commands),
                replies: Some(replies),
                worker: Some(worker),
                lease: None,
                busy: true,
                sequence: 0,
                acquired: 0,
                fault: fault.to_owned(),
                captures: 0,
                publications: 0,
                drops: 0,
                finished: false,
            };
            session.reply()?;
            Ok(session)
        }
        fn gl(&self) -> Gl {
            self.context.as_ref().unwrap().gl()
        }
        fn reply(&mut self) -> Result<bool> {
            require!(self.busy, "no-command-in-flight");
            let received = self
                .replies
                .as_ref()
                .unwrap()
                .recv_timeout(Duration::from_secs(2));
            let reply = match received {
                Ok(reply) => reply,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.worker
                        .take()
                        .unwrap()
                        .join()
                        .map_err(|_| "worker-panic")??;
                    return Err("worker-stopped");
                }
                Err(mpsc::RecvTimeoutError::Timeout) => return Err("worker-reply-timeout"),
            };
            self.busy = false;
            Ok(reply)
        }
        fn submit(&mut self) -> Result<()> {
            require!(!self.lost.load(Ordering::SeqCst), "invalid-session");
            require!(!self.busy, "command-in-flight");
            self.sequence += 1;
            let sizes = [(7, 5), (320, 224), (320, 448), (3, 9)];
            let (mut width, mut height) = sizes[self.sequence as usize % 4];
            let generation = self.pool.lock().unwrap().generation;
            if generation == 1 && self.sequence == 2 {
                width = WIDTH;
                height = HEIGHT;
            }
            if self.fault == "oversize" {
                width = WIDTH + 1;
            }
            let frame = Frame {
                sequence: self.sequence,
                generation,
                width,
                height,
                bottom_left: self.sequence & 4 != 0,
            };
            self.commands
                .as_ref()
                .unwrap()
                .send(Command::Render(frame))
                .map_err(|_| "worker-stopped")?;
            self.busy = true;
            Ok(())
        }
        fn step(&mut self) -> Result<bool> {
            self.submit()?;
            let published = self.reply()?;
            if published {
                self.publications += 1;
            } else {
                self.drops += 1;
            }
            Ok(published)
        }
        fn release(&mut self) -> Result<()> {
            require!(!self.lost.load(Ordering::SeqCst), "invalid-session");
            if let Some(lease) = self.lease.take() {
                let fence = if self.fault == "consumer-fence" {
                    None
                } else {
                    Some(Fence::new(&self.display, self.gl(), false)?)
                };
                let mut pool = self.pool.lock().unwrap();
                let slot = &mut pool.slots[lease.index];
                require!(slot.state == State::Reading, "invalid-lease");
                slot.fence = fence;
                slot.state = State::Retiring;
            }
            Ok(())
        }
        fn acquire(&mut self) -> Result<bool> {
            require!(!self.lost.load(Ordering::SeqCst), "invalid-session");
            let mut completed = [false; 3];
            let mut newest: Option<(usize, Frame, u32)> = None;
            for (i, complete) in completed.iter_mut().enumerate() {
                let (frame, texture, handle) = {
                    let pool = self.pool.lock().unwrap();
                    let slot = &pool.slots[i];
                    if slot.state != State::Ready {
                        continue;
                    }
                    require!(slot.frame.generation == pool.generation, "stale-generation");
                    (
                        slot.frame,
                        slot.texture,
                        slot.fence.as_ref().map_or(0, |f| f.handle),
                    )
                };
                require!(frame.valid(), "unsupported-geometry");
                if self.display.poll(handle)? {
                    *complete = true;
                    if (self.fault == "rewind" || frame.sequence > self.acquired)
                        && newest.map_or(true, |(_, f, _)| frame.sequence > f.sequence)
                    {
                        newest = Some((i, frame, texture));
                    }
                }
            }
            if newest.is_some() {
                self.release()?;
            }
            for (i, complete) in completed.into_iter().enumerate() {
                if !complete {
                    continue;
                }
                let fence = {
                    let mut pool = self.pool.lock().unwrap();
                    pool.slots[i].state = if newest.map(|n| n.0) == Some(i) {
                        State::Reading
                    } else {
                        State::Free
                    };
                    pool.slots[i].fence.take()
                };
                drop(fence);
            }
            if let Some((index, frame, texture)) = newest {
                self.acquired = frame.sequence;
                unsafe {
                    (self.gl().bind_texture)(TEXTURE, texture);
                }
                self.lease = Some(Lease {
                    index,
                    frame,
                    texture,
                });
                Ok(true)
            } else {
                Ok(false)
            }
        }
        fn acquire_tick(&mut self) -> Result<()> {
            for _ in 0..250 {
                if self.acquire()? {
                    return Ok(());
                }
                tick();
            }
            Err("fence-timeout")
        }
        fn capture(&mut self) -> Result<()> {
            require!(!self.lost.load(Ordering::SeqCst), "invalid-session");
            let lease = self.lease.as_ref().ok_or("invalid-session")?;
            let f = lease.frame;
            let gl = self.gl();
            let mut pixels = vec![0u8; (f.width * f.height * 4) as usize];
            unsafe {
                (gl.bind_fbo)(FRAMEBUFFER, self.reader.as_ref().unwrap().fbo);
                (gl.attach)(FRAMEBUFFER, 0x8ce0, TEXTURE, lease.texture, 0);
                require!(
                    (gl.fbo_status)(FRAMEBUFFER) == 0x8cd5,
                    "capture-framebuffer"
                );
                let mut pack = 0;
                (gl.get_integer)(PACK_ALIGNMENT, &mut pack);
                require!(pack == 8, "pack-state-not-preserved");
                (gl.pack)(PACK_ALIGNMENT, 1);
                (gl.read)(
                    0,
                    0,
                    f.width,
                    f.height,
                    0x1908,
                    0x1401,
                    pixels.as_mut_ptr().cast(),
                );
                (gl.pack)(PACK_ALIGNMENT, pack);
                require!((gl.get_error)() == 0, "capture-gl-error");
            }
            for y in 0..f.height {
                for x in 0..f.width {
                    let row = if f.bottom_left { f.height - 1 - y } else { y };
                    let p = &pixels[((row * f.width + x) * 4) as usize..][..4];
                    require!(p[0] == (f.sequence & 255) as u8, "frame-pattern");
                    require!(
                        p[1] == (if y < f.height / 2 { 31 } else { 161 })
                            + ((f.sequence >> 8) & 63) as u8,
                        "frame-pattern"
                    );
                    require!(
                        p[2] == if x < f.width / 2 { 17 } else { 223 },
                        "frame-pattern"
                    );
                    require!(p[3] == 255, "frame-pattern");
                }
            }
            self.captures += 1;
            Ok(())
        }
        fn ready(&self) -> Result<usize> {
            self.pool
                .lock()
                .unwrap()
                .slots
                .iter()
                .position(|s| s.state == State::Ready)
                .ok_or("no-ready-slot")
        }
        fn hold_completion(&mut self, index: usize) -> Result<usize> {
            let handle = self.pool.lock().unwrap().slots[index]
                .fence
                .as_ref()
                .ok_or("missing-fence")?
                .handle;
            wait(&self.display, handle)?;
            let held = Fence::new(&self.display, self.gl(), true)?;
            let token = held.handle;
            let old = self.pool.lock().unwrap().slots[index].fence.replace(held);
            drop(old);
            Ok(token)
        }
        fn pressure(&mut self) -> Result<()> {
            let held = self.lease.as_ref().unwrap().frame.sequence;
            for i in 0..34 {
                self.submit()?;
                self.capture()?;
                let published = self.reply()?;
                require!(published == (i < 2), "pool-capacity-or-lease");
                if published {
                    self.publications += 1;
                } else {
                    self.drops += 1;
                }
                require!(
                    self.lease.as_ref().unwrap().frame.sequence == held,
                    "held-lease-changed"
                );
            }
            self.acquire_tick()?;
            self.capture()?;
            require!(self.acquired == held + 2, "newest-completion");
            Ok(())
        }
        fn pending_checks(&mut self) -> Result<()> {
            require!(self.step()?, "expected-publication");
            let held = self.hold_completion(self.ready()?)?;
            let old = self.acquired;
            for _ in 0..3 {
                require!(
                    !self.acquire()? && self.acquired == old,
                    "pending-producer-reused"
                );
                tick();
            }
            self.capture()?;
            self.display.signal(held)?;
            self.acquire_tick()?;
            self.capture()?;
            require!(self.step()?, "expected-publication");
            let held = self.hold_completion(self.ready()?)?;
            require!(self.step()?, "expected-publication");
            self.acquire_tick()?;
            let newest = self.acquired;
            require!(newest == self.sequence, "newest-completion");
            self.display.signal(held)?;
            require!(
                !self.acquire()? && self.acquired == newest,
                "late-frame-must-not-rewind"
            );
            self.capture()?;
            let retiring = self.lease.as_ref().unwrap().index;
            self.release()?;
            let held = self.hold_completion(retiring)?;
            for _ in 0..2 {
                require!(self.step()?, "expected-publication");
                require!(
                    self.pool.lock().unwrap().slots[retiring].state == State::Retiring,
                    "pending-consumer-reused"
                );
            }
            require!(!self.step()?, "pending-consumer-reused");
            self.display.signal(held)?;
            require!(self.step()?, "expected-publication");
            self.acquire_tick()?;
            self.capture()?;
            Ok(())
        }
        fn finish(&mut self, simulate_loss: bool) -> Result<usize> {
            require!(!self.busy, "shutdown-in-flight");
            if simulate_loss {
                self.lost.store(true, Ordering::SeqCst);
                self.pool.lock().unwrap().generation += 1;
            } else {
                let mut drained = false;
                for _ in 0..250 {
                    self.acquire()?;
                    drained = self
                        .pool
                        .lock()
                        .unwrap()
                        .slots
                        .iter()
                        .all(|s| s.state != State::Ready);
                    if drained {
                        break;
                    }
                    tick();
                }
                require!(drained, "shutdown-ready-timeout");
                unsafe {
                    (self.gl().bind_fbo)(FRAMEBUFFER, 0);
                    (self.gl().bind_texture)(TEXTURE, 0);
                }
                self.reader.take();
                self.release()?;
            }
            self.commands
                .as_ref()
                .unwrap()
                .send(Command::Stop)
                .map_err(|_| "worker-stopped")?;
            self.commands.take();
            self.replies.take();
            self.worker
                .take()
                .unwrap()
                .join()
                .map_err(|_| "worker-panic")??;
            self.clear_fences();
            self.reader.take();
            self.context.take();
            self.lease.take();
            let created = self.display.created.load(Ordering::SeqCst);
            require!(
                created == self.display.destroyed.load(Ordering::SeqCst),
                "unbalanced-fences"
            );
            self.finished = true;
            Ok(created)
        }
        fn clear_fences(&mut self) {
            let fences: [Option<Fence>; 3] = {
                let mut pool = self.pool.lock().unwrap();
                std::array::from_fn(|i| pool.slots[i].fence.take())
            };
            drop(fences);
        }
    }
    impl Drop for Session {
        fn drop(&mut self) {
            if !self.finished {
                // An early return cannot pretend that a dropped CPU lease makes
                // GPU storage reusable. Stop the entire synthetic share group.
                self.lost.store(true, Ordering::SeqCst);
                self.pool.lock().unwrap().generation += 1;
                self.commands.take();
                self.replies.take();
                if let Some(worker) = self.worker.take() {
                    let _ = worker.join();
                }
                self.clear_fences();
                self.reader.take();
                self.context.take();
            }
        }
    }
    pub fn live_resources() -> bool {
        [
            CONTEXTS.load(Ordering::SeqCst),
            SURFACES.load(Ordering::SeqCst),
            DISPLAYS.load(Ordering::SeqCst),
            SYNCS.load(Ordering::SeqCst),
        ]
        .iter()
        .any(|&n| n != 0)
    }
    pub fn cleanup_failed() -> bool {
        CLEANUP_FAILED.load(Ordering::SeqCst)
    }

    fn startup_checks() -> Result<()> {
        for (stage, reason) in [
            ("surface", "startup-surface"),
            ("context", "startup-context"),
            ("current", "startup-current"),
        ] {
            let result = Context::new(Display::new()?, 0, 3, stage);
            require!(result.err() == Some(reason), "startup-control-not-detected");
            require!(!live_resources(), "startup-resource-leak");
        }
        // Intentional unwind is contained on this thread, with no live worker.
        let unwind = std::panic::catch_unwind(|| {
            let context = Context::new(Display::new().unwrap(), 0, 3, "").unwrap();
            let lost = Arc::new(AtomicBool::new(false));
            let _target = Target::new(context.gl(), &lost, true).unwrap();
            panic!("authored startup unwind");
        });
        let authored_unwind = match unwind {
            Err(payload) => payload.downcast_ref::<&str>() == Some(&"authored startup unwind"),
            Ok(_) => false,
        };
        require!(
            authored_unwind && !live_resources() && !cleanup_failed(),
            "unwind-cleanup-failure"
        );
        // Both paths leave a held lease and two Ready slots. Drop must stop and
        // join the worker without reusing or GL-deleting those leased textures.
        for panic_in_session in [false, true] {
            let outcome = std::panic::catch_unwind(|| -> Result<()> {
                let mut session = Session::new(2, "")?;
                require!(session.step()?, "expected-publication");
                session.acquire_tick()?;
                session.capture()?;
                for _ in 0..2 {
                    require!(session.step()?, "expected-publication");
                }
                if panic_in_session {
                    panic!("authored active-worker unwind");
                }
                Ok(())
            });
            match (panic_in_session, outcome) {
                (false, Ok(result)) => result?,
                (true, Err(payload))
                    if payload.downcast_ref::<&str>() == Some(&"authored active-worker unwind") =>
                {
                    ()
                }
                _ => return Err("active-worker-control-failure"),
            }
            require!(
                !live_resources() && !cleanup_failed(),
                "early-return-cleanup-failure"
            );
        }
        println!("PASS partial-startup=3 startup-unwind=1 active-worker-early-return=1 active-worker-unwind=1");
        Ok(())
    }
    pub fn run() -> Result<()> {
        require!(
            std::env::var("EGL_PLATFORM").as_deref() == Ok("surfaceless"),
            "surfaceless-host-required"
        );
        let args: Vec<_> = std::env::args().skip(1).collect();
        let faults = [
            "producer-fence",
            "consumer-fence",
            "generation",
            "origin",
            "timeout",
            "oversize",
            "publish-after-loss",
            "capture-after-loss",
            "rewind",
        ];
        let (quick, fault) = match args.as_slice() {
            [] => (false, ""),
            [q] if q == "--quick" => (true, ""),
            [flag, fault] if flag == "--fault" && faults.contains(&fault.as_str()) => {
                (true, fault.as_str())
            }
            _ => return Err("invalid-arguments"),
        };
        if fault.is_empty() {
            startup_checks()?;
        }
        let sessions = if quick { 1 } else { 101 };
        let (mut captures, mut publications, mut drops, mut losses, mut syncs) = (0, 0, 0, 0, 0);
        for generation in 1..=sessions {
            let mut session = Session::new(generation, fault)?;
            let frames = if quick {
                1
            } else if generation == 1 {
                10000
            } else {
                16
            };
            let mut done = 0;
            while done < frames {
                if session.step()? {
                    session.acquire_tick()?;
                    session.capture()?;
                    done += 1;
                } else {
                    tick();
                }
            }
            session.pressure()?;
            session.pending_checks()?;
            for _ in 0..2 {
                require!(session.step()?, "expected-publication");
            }
            let loss = generation % 10 == 0 || fault.ends_with("after-loss");
            syncs += session.finish(loss)?;
            if fault == "publish-after-loss" {
                session.submit()?;
            }
            if fault == "capture-after-loss" {
                session.capture()?;
            }
            captures += session.captures;
            publications += session.publications;
            drops += session.drops;
            losses += usize::from(loss);
            drop(session);
            require!(
                !live_resources() && !cleanup_failed(),
                "lifecycle-resource-leak"
            );
        }
        println!("PASS sessions={sessions} captures={captures} publications={publications} publication_drops={drops} simulated_losses={losses} balanced_syncs={syncs} slots=3 stable_fbo=1024x512");
        println!(
            "SCOPE=Rust surfaceless synthetic correctness; no Swan/window/device/speed acceptance"
        );
        Ok(())
    }
}
