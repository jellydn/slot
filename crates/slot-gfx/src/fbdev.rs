use std::ffi::{c_char, c_void, CString};

use libloading::Library;

use crate::surface::{GfxError, Surface};

const FALLBACK_PANEL: (u32, u32) = (720, 480);

const FB0: &str = "/sys/class/graphics/fb0";

const EGL_SUCCESS: i32 = 0x3000;
const EGL_NONE: i32 = 0x3038;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_HEIGHT: i32 = 0x3056;
const EGL_WIDTH: i32 = 0x3057;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_WINDOW_BIT: i32 = 0x0004;
const EGL_OPENGL_ES2_BIT: i32 = 0x0004;

type Ptr = *mut c_void;
type GetDisplay = unsafe extern "C" fn(Ptr) -> Ptr;
type Initialize = unsafe extern "C" fn(Ptr, *mut i32, *mut i32) -> u32;
type BindApi = unsafe extern "C" fn(u32) -> u32;
type ChooseConfig = unsafe extern "C" fn(Ptr, *const i32, *mut Ptr, i32, *mut i32) -> u32;
type CreateWindowSurface = unsafe extern "C" fn(Ptr, Ptr, Ptr, *const i32) -> Ptr;
type CreateContext = unsafe extern "C" fn(Ptr, Ptr, Ptr, *const i32) -> Ptr;
type MakeCurrent = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> u32;
type QuerySurface = unsafe extern "C" fn(Ptr, Ptr, i32, *mut i32) -> u32;
type SwapBuffers = unsafe extern "C" fn(Ptr, Ptr) -> u32;
type SwapInterval = unsafe extern "C" fn(Ptr, i32) -> u32;
type GetProcAddress = unsafe extern "C" fn(*const c_char) -> *const c_void;
type GetError = unsafe extern "C" fn() -> i32;
type Terminate = unsafe extern "C" fn(Ptr) -> u32;
type Destroy = unsafe extern "C" fn(Ptr, Ptr) -> u32;
type ReleaseThread = unsafe extern "C" fn() -> u32;

struct Egl {
    get_display: GetDisplay,
    initialize: Initialize,
    bind_api: BindApi,
    choose_config: ChooseConfig,
    create_window_surface: CreateWindowSurface,
    create_context: CreateContext,
    make_current: MakeCurrent,
    query_surface: QuerySurface,
    swap_buffers: SwapBuffers,
    swap_interval: SwapInterval,
    get_proc_address: GetProcAddress,
    get_error: GetError,
    terminate: Terminate,
    destroy_context: Destroy,
    destroy_surface: Destroy,
    release_thread: ReleaseThread,
    _lib: Library,
}

#[repr(C)]
struct FbdevWindow {
    width: u16,
    height: u16,
}

pub struct FbdevSurface {
    egl: Egl,
    gles: Library,
    display: Ptr,
    surface: Ptr,
    context: Ptr,
    size: (u32, u32),
    initialized: bool,
    touched: bool,
    _window: Option<Box<FbdevWindow>>,
}

#[cfg(unix)]
const FBIOGET_VSCREENINFO: std::ffi::c_ulong = 0x4600;

#[cfg(unix)]
extern "C" {
    fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
}

fn visible_size() -> Option<(u32, u32)> {
    #[cfg(not(unix))]
    {
        None
    }
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fb = std::fs::File::open("/dev/fb0").ok()?;
        let mut info = [0u32; 40];
        let rc = unsafe { ioctl(fb.as_raw_fd(), FBIOGET_VSCREENINFO, info.as_mut_ptr()) };
        (rc == 0 && info[0] > 0 && info[1] > 0).then_some((info[0], info[1]))
    }
}

pub fn panel_mode(text: &str) -> Option<(u32, u32)> {
    let body = text.lines().next()?.rsplit(':').next()?;
    let (w, rest) = body.split_once('x')?;
    let h: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let (w, h) = (w.trim().parse().ok()?, h.parse().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

pub fn panel_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.trim().split_once(',')?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

pub fn egl_error(what: &str, code: i32) -> GfxError {
    match code {
        EGL_SUCCESS => GfxError::Context(format!("{what}: refused, egl flagged nothing")),
        _ => GfxError::Context(format!("{what}: egl error 0x{code:x}")),
    }
}

fn open(name: &str) -> Result<Library, GfxError> {
    unsafe { Library::new(name) }.map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T, GfxError> {
    lib.get::<T>(name.as_bytes())
        .map(|s| *s)
        .map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

impl Egl {
    fn load() -> Result<Self, GfxError> {
        #[cfg(feature = "stock-tg5040")]
        let lib = open("/usr/lib/libEGL.so.1").or_else(|_| open("/usr/lib/libEGL.so"))?;
        #[cfg(not(feature = "stock-tg5040"))]
        let lib = open("libEGL.so.1").or_else(|_| open("libEGL.so"))?;
        unsafe {
            Ok(Egl {
                get_display: sym(&lib, "eglGetDisplay")?,
                initialize: sym(&lib, "eglInitialize")?,
                bind_api: sym(&lib, "eglBindAPI")?,
                choose_config: sym(&lib, "eglChooseConfig")?,
                create_window_surface: sym(&lib, "eglCreateWindowSurface")?,
                create_context: sym(&lib, "eglCreateContext")?,
                make_current: sym(&lib, "eglMakeCurrent")?,
                query_surface: sym(&lib, "eglQuerySurface")?,
                swap_buffers: sym(&lib, "eglSwapBuffers")?,
                swap_interval: sym(&lib, "eglSwapInterval")?,
                get_proc_address: sym(&lib, "eglGetProcAddress")?,
                get_error: sym(&lib, "eglGetError")?,
                terminate: sym(&lib, "eglTerminate")?,
                destroy_context: sym(&lib, "eglDestroyContext")?,
                destroy_surface: sym(&lib, "eglDestroySurface")?,
                release_thread: sym(&lib, "eglReleaseThread")?,
                _lib: lib,
            })
        }
    }

    fn fail(&self, what: &str) -> GfxError {
        egl_error(what, unsafe { (self.get_error)() })
    }

    fn check(&self, what: &str, ok: bool) -> Result<(), GfxError> {
        let code = unsafe { (self.get_error)() };
        if !ok || code != EGL_SUCCESS {
            return Err(egl_error(what, code));
        }
        Ok(())
    }
}

impl FbdevSurface {
    pub fn new() -> Result<Self, GfxError> {
        if cfg!(feature = "stock-tg5040") {
            // Surface dimensions are queried below; do not open fb0 for a hint.
            return Self::open((1280, 720));
        }
        let attr = |name: &str| std::fs::read_to_string(format!("{FB0}/{name}")).ok();
        let hint = attr("mode")
            .as_deref()
            .and_then(panel_mode)
            .or_else(|| attr("modes").as_deref().and_then(panel_mode))
            .or_else(visible_size)
            .or_else(|| attr("virtual_size").as_deref().and_then(panel_size))
            .unwrap_or(FALLBACK_PANEL);
        eprintln!("slot: fb0 hint {}x{}", hint.0, hint.1);
        FbdevSurface::open(hint)
    }

    fn open(hint: (u32, u32)) -> Result<Self, GfxError> {
        let egl = Egl::load()?;
        #[cfg(feature = "stock-tg5040")]
        let gles = open("/usr/lib/libGLESv2.so.2").or_else(|_| open("/usr/lib/libGLESv2.so"))?;
        #[cfg(not(feature = "stock-tg5040"))]
        let gles = open("libGLESv2.so.2").or_else(|_| open("libGLESv2.so"))?;
        Self::open_with(egl, gles, hint)
    }

    fn open_with(egl: Egl, gles: Library, hint: (u32, u32)) -> Result<Self, GfxError> {
        // Own each resource immediately, so every early return runs the same Drop.
        let mut out = Self {
            egl,
            gles,
            display: std::ptr::null_mut(),
            surface: std::ptr::null_mut(),
            context: std::ptr::null_mut(),
            size: hint,
            initialized: false,
            touched: false,
            _window: None,
        };
        unsafe {
            out.touched = true;
            out.display = (out.egl.get_display)(std::ptr::null_mut());
            out.egl.check("eglGetDisplay", !out.display.is_null())?;
            let rc = (out.egl.initialize)(out.display, std::ptr::null_mut(), std::ptr::null_mut());
            out.initialized = rc != 0;
            out.egl.check("eglInitialize", out.initialized)?;
            let rc = (out.egl.bind_api)(EGL_OPENGL_ES_API);
            out.egl.check("eglBindAPI", rc != 0)?;
            let attrs = [
                EGL_SURFACE_TYPE,
                EGL_WINDOW_BIT,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                0,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES2_BIT,
                EGL_NONE,
            ];
            let mut config: Ptr = std::ptr::null_mut();
            let mut found = 0;
            let rc =
                (out.egl.choose_config)(out.display, attrs.as_ptr(), &mut config, 1, &mut found);
            out.egl.check("eglChooseConfig", rc != 0)?;
            if found < 1 || config.is_null() {
                return Err(GfxError::Context(
                    "eglChooseConfig: no es2 window config with 8/8/8 colour".into(),
                ));
            }
            // Keep the existing native-window-first order on non-stock devices.
            if !cfg!(feature = "stock-tg5040") {
                out._window = Some(Box::new(FbdevWindow {
                    width: hint.0 as u16,
                    height: hint.1 as u16,
                }));
            }
            let native = out
                ._window
                .as_deref_mut()
                .map_or(std::ptr::null_mut(), |window| {
                    window as *mut FbdevWindow as Ptr
                });
            out.surface =
                (out.egl.create_window_surface)(out.display, config, native, std::ptr::null());
            let first = out
                .egl
                .check("eglCreateWindowSurface", !out.surface.is_null());
            if cfg!(feature = "stock-tg5040") || !out.surface.is_null() {
                first?;
            } else {
                out.surface = (out.egl.create_window_surface)(
                    out.display,
                    config,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                );
                out.egl.check(
                    "eglCreateWindowSurface(NULL fallback)",
                    !out.surface.is_null(),
                )?;
            }
            let context_attrs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
            out.context = (out.egl.create_context)(
                out.display,
                config,
                std::ptr::null_mut(),
                context_attrs.as_ptr(),
            );
            out.egl.check("eglCreateContext", !out.context.is_null())?;
            let rc = (out.egl.make_current)(out.display, out.surface, out.surface, out.context);
            out.egl.check("eglMakeCurrent", rc != 0)?;
            let rc = (out.egl.swap_interval)(out.display, 1);
            out.egl.check("eglSwapInterval", rc != 0)?;
            out.size = query_size(&out.egl, out.display, out.surface)?;
            Ok(out)
        }
    }
}

fn query_size(egl: &Egl, display: Ptr, surface: Ptr) -> Result<(u32, u32), GfxError> {
    let (mut w, mut h) = (0, 0);
    unsafe {
        let rc = (egl.query_surface)(display, surface, EGL_WIDTH, &mut w);
        egl.check("eglQuerySurface(width)", rc != 0)?;
        let rc = (egl.query_surface)(display, surface, EGL_HEIGHT, &mut h);
        egl.check("eglQuerySurface(height)", rc != 0)?;
    }
    if w <= 0 || h <= 0 {
        return Err(GfxError::Context(
            "eglQuerySurface: invalid dimensions".into(),
        ));
    }
    Ok((w as u32, h as u32))
}

impl Surface for FbdevSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        let ok = unsafe {
            (self.egl.make_current)(self.display, self.surface, self.surface, self.context)
        };
        match ok {
            0 => Err(self.egl.fail("eglMakeCurrent")),
            _ => Ok(()),
        }
    }

    fn window_size(&self) -> (u32, u32) {
        self.size
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        match unsafe { (self.egl.swap_buffers)(self.display, self.surface) } {
            0 => Err(self.egl.fail("eglSwapBuffers")),
            _ => Ok(()),
        }
    }

    fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(c) = CString::new(name) else {
            return std::ptr::null();
        };
        let exported = unsafe { self.gles.get::<unsafe extern "C" fn()>(name.as_bytes()) };
        match exported {
            Ok(f) => *f as *const c_void,
            Err(_) => unsafe { (self.egl.get_proc_address)(c.as_ptr()) },
        }
    }
}

impl Drop for FbdevSurface {
    fn drop(&mut self) {
        unsafe {
            let release = |name, rc| {
                if let Err(e) = self.egl.check(name, rc != 0) {
                    eprintln!("slot: cleanup: {e}");
                }
            };
            if self.initialized && (!self.surface.is_null() || !self.context.is_null()) {
                release(
                    "eglMakeCurrent(unbind)",
                    (self.egl.make_current)(
                        self.display,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    ),
                );
            }
            if !self.context.is_null() {
                release(
                    "eglDestroyContext",
                    (self.egl.destroy_context)(self.display, self.context),
                );
            }
            if !self.surface.is_null() {
                release(
                    "eglDestroySurface",
                    (self.egl.destroy_surface)(self.display, self.surface),
                );
            }
            if self.initialized {
                release("eglTerminate", (self.egl.terminate)(self.display));
            }
            if self.touched {
                release("eglReleaseThread", (self.egl.release_thread)());
            }
        }
    }
}

#[cfg(all(test, unix, feature = "stock-tg5040"))]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Driver {
        calls: usize,
        fail: usize,
        error: i32,
        initialized: bool,
        surface: bool,
        context: bool,
        non_null_window: bool,
        cleanup_failure: bool,
        terminated: bool,
        released: bool,
    }
    thread_local! { static DRIVER: RefCell<Driver> = RefCell::new(Driver::default()); }
    fn step() -> bool {
        DRIVER.with(|state| {
            let mut s = state.borrow_mut();
            s.calls += 1;
            let ok = s.calls != s.fail;
            s.error = if ok { EGL_SUCCESS } else { 0x3003 };
            ok
        })
    }
    fn handle(ok: bool) -> Ptr {
        if ok {
            1usize as Ptr
        } else {
            std::ptr::null_mut()
        }
    }
    unsafe extern "C" fn display(_: Ptr) -> Ptr {
        handle(step())
    }
    unsafe extern "C" fn initialize(_: Ptr, _: *mut i32, _: *mut i32) -> u32 {
        let ok = step();
        DRIVER.with(|s| s.borrow_mut().initialized = ok);
        u32::from(ok)
    }
    unsafe extern "C" fn bind(_: u32) -> u32 {
        u32::from(step())
    }
    unsafe extern "C" fn choose(
        _: Ptr,
        _: *const i32,
        config: *mut Ptr,
        _: i32,
        n: *mut i32,
    ) -> u32 {
        let ok = step();
        *config = handle(ok);
        *n = i32::from(ok);
        u32::from(ok)
    }
    unsafe extern "C" fn surface(_: Ptr, _: Ptr, window: Ptr, _: *const i32) -> Ptr {
        let ok = step();
        DRIVER.with(|s| {
            let mut s = s.borrow_mut();
            s.surface = ok;
            s.non_null_window |= !window.is_null();
        });
        handle(ok)
    }
    unsafe extern "C" fn context(_: Ptr, _: Ptr, _: Ptr, _: *const i32) -> Ptr {
        let ok = step();
        DRIVER.with(|s| s.borrow_mut().context = ok);
        handle(ok)
    }
    unsafe extern "C" fn current(_: Ptr, _: Ptr, _: Ptr, _: Ptr) -> u32 {
        u32::from(step())
    }
    unsafe extern "C" fn query(_: Ptr, _: Ptr, attr: i32, value: *mut i32) -> u32 {
        let ok = step();
        *value = if attr == EGL_WIDTH { 1280 } else { 720 };
        u32::from(ok)
    }
    unsafe extern "C" fn swap(_: Ptr, _: Ptr) -> u32 {
        u32::from(step())
    }
    unsafe extern "C" fn interval(_: Ptr, _: i32) -> u32 {
        u32::from(step())
    }
    unsafe extern "C" fn proc(_: *const c_char) -> *const c_void {
        std::ptr::null()
    }
    unsafe extern "C" fn error() -> i32 {
        DRIVER.with(|s| {
            let mut s = s.borrow_mut();
            let e = s.error;
            s.error = EGL_SUCCESS;
            e
        })
    }
    unsafe extern "C" fn destroy_context(_: Ptr, _: Ptr) -> u32 {
        let ok = step() && !DRIVER.with(|s| s.borrow().cleanup_failure);
        DRIVER.with(|s| s.borrow_mut().context = !ok);
        u32::from(ok)
    }
    unsafe extern "C" fn destroy_surface(_: Ptr, _: Ptr) -> u32 {
        let ok = step();
        DRIVER.with(|s| s.borrow_mut().surface = !ok);
        u32::from(ok)
    }
    unsafe extern "C" fn terminate(_: Ptr) -> u32 {
        let ok = step();
        DRIVER.with(|s| {
            let mut s = s.borrow_mut();
            s.initialized = !ok;
            s.terminated = true;
        });
        u32::from(ok)
    }
    unsafe extern "C" fn release() -> u32 {
        let ok = step();
        DRIVER.with(|s| s.borrow_mut().released = true);
        u32::from(ok)
    }
    fn mock() -> Egl {
        Egl {
            get_display: display,
            initialize,
            bind_api: bind,
            choose_config: choose,
            create_window_surface: surface,
            create_context: context,
            make_current: current,
            query_surface: query,
            swap_buffers: swap,
            swap_interval: interval,
            get_proc_address: proc,
            get_error: error,
            terminate,
            destroy_context,
            destroy_surface,
            release_thread: release,
            _lib: libloading::os::unix::Library::this().into(),
        }
    }

    #[test]
    fn stock_null_window_and_cleanup_on_each_initialization_failure() {
        // Ten fallible initialization calls, including both surface dimensions.
        for fail in 0..=10 {
            DRIVER.with(|s| {
                *s.borrow_mut() = Driver {
                    fail,
                    ..Default::default()
                }
            });
            let result = FbdevSurface::open_with(
                mock(),
                libloading::os::unix::Library::this().into(),
                (1, 1),
            );
            if fail == 0 {
                assert_eq!(result.as_ref().unwrap().window_size(), (1280, 720));
            } else {
                assert!(result.is_err(), "failure {fail} must be propagated");
            }
            drop(result);
            DRIVER.with(|s| {
                let s = s.borrow();
                assert!(
                    !s.initialized && !s.context && !s.surface,
                    "failure {fail} leaked owned resources"
                );
                assert!(!s.non_null_window);
                assert!(s.released);
            });
        }
        DRIVER.with(|s| {
            *s.borrow_mut() = Driver {
                cleanup_failure: true,
                ..Default::default()
            }
        });
        drop(
            FbdevSurface::open_with(mock(), libloading::os::unix::Library::this().into(), (1, 1))
                .unwrap(),
        );
        DRIVER.with(|s| {
            let s = s.borrow();
            assert!(s.context); // Destroy failure is not reported as successful release.
            assert!(!s.surface && s.terminated && s.released);
        });
    }
}
