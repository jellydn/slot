//! Diagnostic-only EGL ownership. No native window, display swap or surface fallback.
use libloading::Library;
use std::ffi::{c_char, c_void, CString};
use std::ptr;

type Handle = *mut c_void;
type MakeCurrent = unsafe extern "C" fn(Handle, Handle, Handle, Handle) -> u32;
type Destroy = unsafe extern "C" fn(Handle, Handle) -> u32;
type Terminate = unsafe extern "C" fn(Handle) -> u32;
type GetError = unsafe extern "C" fn() -> u32;
type GetProc = unsafe extern "C" fn(*const c_char) -> *const c_void;

pub struct Pbuffer {
    display: Handle,
    surface: Handle,
    context: Handle,
    initialized: bool,
    make_current: MakeCurrent,
    destroy_context: Destroy,
    destroy_surface: Destroy,
    terminate: Terminate,
    get_error: GetError,
    get_proc: GetProc,
    gles: Library,
    _egl: Library,
}

impl Pbuffer {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let egl = Library::new("/usr/lib/libEGL.so").map_err(|e| e.to_string())?;
            let gles = Library::new("/usr/lib/libGLESv2.so").map_err(|e| e.to_string())?;
            macro_rules! load {
                ($name:literal, $ty:ty) => {
                    *egl.get::<$ty>(concat!($name, "\0").as_bytes())
                        .map_err(|e| e.to_string())?
                };
            }
            let get_display = load!("eglGetDisplay", unsafe extern "C" fn(Handle) -> Handle);
            let initialize = load!(
                "eglInitialize",
                unsafe extern "C" fn(Handle, *mut i32, *mut i32) -> u32
            );
            let bind_api = load!("eglBindAPI", unsafe extern "C" fn(u32) -> u32);
            let choose = load!(
                "eglChooseConfig",
                unsafe extern "C" fn(Handle, *const i32, *mut Handle, i32, *mut i32) -> u32
            );
            let create_surface = load!(
                "eglCreatePbufferSurface",
                unsafe extern "C" fn(Handle, Handle, *const i32) -> Handle
            );
            let create_context = load!(
                "eglCreateContext",
                unsafe extern "C" fn(Handle, Handle, Handle, *const i32) -> Handle
            );
            let mut owner = Self {
                display: ptr::null_mut(),
                surface: ptr::null_mut(),
                context: ptr::null_mut(),
                initialized: false,
                make_current: load!("eglMakeCurrent", MakeCurrent),
                destroy_context: load!("eglDestroyContext", Destroy),
                destroy_surface: load!("eglDestroySurface", Destroy),
                terminate: load!("eglTerminate", Terminate),
                get_error: load!("eglGetError", GetError),
                get_proc: load!("eglGetProcAddress", GetProc),
                gles,
                _egl: egl,
            };
            owner.display = get_display(ptr::null_mut());
            owner.check(!owner.display.is_null(), "get display")?;
            let (mut major, mut minor) = (0, 0);
            owner.check(
                initialize(owner.display, &mut major, &mut minor) != 0,
                "initialize",
            )?;
            owner.initialized = true;
            owner.check(bind_api(0x30a0) != 0, "bind ES API")?;
            // RGBA8, ES3, pbuffer only. No window bit or window creation function.
            let attrs = [
                0x3033, 1, 0x3040, 0x40, 0x3024, 8, 0x3023, 8, 0x3022, 8, 0x3021, 8, 0x3038,
            ];
            let mut config = ptr::null_mut();
            let mut count = 0;
            owner.check(
                choose(owner.display, attrs.as_ptr(), &mut config, 1, &mut count) != 0
                    && count == 1
                    && !config.is_null(),
                "choose ES3 pbuffer",
            )?;
            let size = [0x3057, 64, 0x3056, 64, 0x3038];
            owner.surface = create_surface(owner.display, config, size.as_ptr());
            owner.check(!owner.surface.is_null(), "create pbuffer")?;
            // Request ES3.2 explicitly, not an ES2 context with ambiguous negotiation.
            let version = [0x3098, 3, 0x30fb, 2, 0x3038];
            owner.context =
                create_context(owner.display, config, ptr::null_mut(), version.as_ptr());
            owner.check(!owner.context.is_null(), "create ES3.2 context")?;
            owner.make_current()?;
            Ok(owner)
        }
    }

    fn check(&self, success: bool, operation: &str) -> Result<(), String> {
        if success {
            Ok(())
        } else {
            Err(format!("EGL {operation}: {:#x}", unsafe {
                (self.get_error)()
            }))
        }
    }

    pub fn make_current(&mut self) -> Result<(), String> {
        self.check(
            unsafe {
                (self.make_current)(self.display, self.surface, self.surface, self.context) != 0
            },
            "make current",
        )
    }

    pub fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(name) = CString::new(name) else {
            return ptr::null();
        };
        unsafe {
            if let Ok(symbol) = self.gles.get::<*const c_void>(name.as_bytes_with_nul()) {
                return *symbol;
            }
            (self.get_proc)(name.as_ptr())
        }
    }
}

impl Drop for Pbuffer {
    fn drop(&mut self) {
        unsafe {
            if self.initialized {
                let detached = (self.make_current)(
                    self.display,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                let context = self.context.is_null()
                    || (self.destroy_context)(self.display, self.context) != 0;
                let surface = self.surface.is_null()
                    || (self.destroy_surface)(self.display, self.surface) != 0;
                let terminated = (self.terminate)(self.display) != 0;
                eprintln!("diagnostic EGL cleanup: detach={} context={context} surface={surface} terminate={terminated}", detached != 0);
                assert!(
                    detached != 0 && context && surface && terminated,
                    "diagnostic EGL cleanup failed"
                );
            }
        }
    }
}
