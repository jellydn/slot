/* Original bounded capability diagnostic. No window/swap/game/file-write API. */
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

typedef void *Handle;
typedef void (*Proc)(void);
typedef unsigned int UInt;
typedef int Int;
#define FN(ret, name, args) static ret (*name) args
FN(Handle, eglGetDisplay, (Handle));
FN(UInt, eglInitialize, (Handle, Int *, Int *));
FN(UInt, eglBindAPI, (UInt));
FN(const char *, eglQueryString, (Handle, Int));
FN(UInt, eglChooseConfig, (Handle, const Int *, Handle *, Int, Int *));
FN(UInt, eglGetConfigAttrib, (Handle, Handle, Int, Int *));
FN(Handle, eglCreatePbufferSurface, (Handle, Handle, const Int *));
FN(Handle, eglCreateContext, (Handle, Handle, Handle, const Int *));
FN(UInt, eglMakeCurrent, (Handle, Handle, Handle, Handle));
FN(UInt, eglDestroyContext, (Handle, Handle));
FN(UInt, eglDestroySurface, (Handle, Handle));
FN(UInt, eglTerminate, (Handle));
FN(Int, eglGetError, (void));
FN(Proc, eglGetProcAddress, (const char *));
FN(const unsigned char *, glGetString, (UInt));
FN(const unsigned char *, glGetStringi, (UInt, UInt));
FN(void, glGetIntegerv, (UInt, Int *));
FN(UInt, glGetError, (void));
FN(void, glGenTextures, (Int, UInt *));
FN(void, glBindTexture, (UInt, UInt));
FN(void, glTexParameteri, (UInt, UInt, Int));
FN(void, glTexImage2D, (UInt, Int, Int, Int, Int, Int, UInt, UInt, const void *));
FN(void, glDeleteTextures, (Int, const UInt *));
FN(void, glGenFramebuffers, (Int, UInt *));
FN(void, glBindFramebuffer, (UInt, UInt));
FN(void, glFramebufferTexture2D, (UInt, UInt, UInt, UInt, Int));
FN(UInt, glCheckFramebufferStatus, (UInt));
FN(void, glDeleteFramebuffers, (Int, const UInt *));
FN(void, glEnable, (UInt));
FN(void, glDisable, (UInt));
FN(void, glScissor, (Int, Int, Int, Int));
FN(void, glClearColor, (float, float, float, float));
FN(void, glClearDepthf, (float));
FN(void, glClear, (UInt));
FN(void, glPixelStorei, (UInt, Int));
FN(void, glReadPixels, (Int, Int, Int, Int, UInt, UInt, void *));

static int extension(const char *list, const char *name) {
    if (!list) return 0;
    size_t n = strlen(name);
    for (const char *p = list; (p = strstr(p, name)); p += n)
        if ((p == list || p[-1] == ' ') && (p[n] == 0 || p[n] == ' ')) return 1;
    return 0;
}

static const char *text(const void *value) { return value ? (const char *)value : "<null>"; }

/* POSIX dlsym and the target's EGL function pointers use pointer-sized ABI slots. */
#define LOAD(lib, name) do { \
    void *symbol = dlsym(lib, #name); \
    if (!symbol && eglGetProcAddress && lib == gles) { \
        Proc proc = eglGetProcAddress(#name); \
        _Static_assert(sizeof(proc) == sizeof(symbol), "function pointer ABI"); \
        memcpy(&symbol, &proc, sizeof(symbol)); \
    } \
    if (!symbol) { printf("missing_symbol=%s\n", #name); goto cleanup; } \
    _Static_assert(sizeof(name) == sizeof(symbol), "function slot ABI"); \
    memcpy(&name, &symbol, sizeof(symbol)); \
} while (0)
#define CHECK(value, label) do { if (!(value)) { printf("failure=%s egl_error=0x%x\n", label, eglGetError()); goto cleanup; } } while (0)

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    /* Default SIGALRM terminates even if normal driver cleanup cannot run. */
    alarm(10);
    void *egl = NULL, *gles = NULL;
    Handle display = NULL, config = NULL, surface = NULL, context = NULL;
    int initialized = 0, current = 0, gl_ready = 0, ok = 0;
    UInt textures[2] = {0, 0}, fbo = 0;
    printf("probe=stock-offscreen-capability-v1 alarm_seconds=10\n");
    egl = dlopen("/usr/lib/libEGL.so", RTLD_NOW | RTLD_LOCAL);
    gles = dlopen("/usr/lib/libGLESv2.so", RTLD_NOW | RTLD_LOCAL);
    if (!egl || !gles) { printf("failure=dlopen %s\n", text(dlerror())); goto cleanup; }
    LOAD(egl, eglGetDisplay); LOAD(egl, eglInitialize); LOAD(egl, eglBindAPI);
    LOAD(egl, eglQueryString); LOAD(egl, eglChooseConfig); LOAD(egl, eglGetConfigAttrib);
    LOAD(egl, eglCreatePbufferSurface); LOAD(egl, eglCreateContext); LOAD(egl, eglMakeCurrent);
    LOAD(egl, eglDestroyContext); LOAD(egl, eglDestroySurface); LOAD(egl, eglTerminate);
    LOAD(egl, eglGetError); LOAD(egl, eglGetProcAddress);
    display = eglGetDisplay(NULL);
    CHECK(display, "get_display");
    Int major = 0, minor = 0;
    CHECK(eglInitialize(display, &major, &minor), "initialize");
    initialized = 1;
    printf("egl_initialized=%d.%d\negl_vendor=%.256s\negl_version=%.256s\negl_client_apis=%.256s\n",
           major, minor, text(eglQueryString(display, 0x3053)),
           text(eglQueryString(display, 0x3054)), text(eglQueryString(display, 0x308d)));
    const char *extensions = eglQueryString(display, 0x3055);
    printf("egl_extensions=%.8192s\n", text(extensions));
    int surfaceless = extension(extensions, "EGL_KHR_surfaceless_context");
    CHECK(eglBindAPI(0x30a0), "bind_es_api");
    Int attrs[] = {0x3033, 1, 0x3040, 0x40, 0x3024, 8, 0x3023, 8,
                   0x3022, 8, 0x3021, 8, 0x3038};
    Int count = 0;
    CHECK(eglChooseConfig(display, attrs, &config, 1, &count), "choose_es3_pbuffer");
    printf("es3_pbuffer_configs_returned=%d\n", count);
    if (count > 0 && config) {
        const Int size[] = {0x3057, 64, 0x3056, 64, 0x3038};
        surface = eglCreatePbufferSurface(display, config, size);
        if (!surface) printf("pbuffer_failed_egl_error=0x%x\n", eglGetError());
    }
    if (!surface) {
        if (!surfaceless) { printf("failure=no_offscreen_surface_and_no_advertised_surfaceless\n"); goto cleanup; }
        attrs[1] = 0; /* No surface-type bits required; never request a window. */
        count = 0; config = NULL;
        CHECK(eglChooseConfig(display, attrs, &config, 1, &count), "choose_es3_surfaceless");
        CHECK(count > 0 && config, "no_es3_config");
    }
    Int bits = 0, types = 0;
    CHECK(eglGetConfigAttrib(display, config, 0x3040, &bits), "config_renderable_type");
    CHECK(eglGetConfigAttrib(display, config, 0x3033, &types), "config_surface_type");
    printf("chosen_renderable_type=0x%x chosen_surface_type=0x%x mode=%s\n",
           bits, types, surface ? "pbuffer64x64" : "advertised_surfaceless");
    const Int context_attrs[] = {0x3098, 3, 0x3038};
    context = eglCreateContext(display, config, NULL, context_attrs);
    CHECK(context, "create_es3_context");
    CHECK(eglMakeCurrent(display, surface, surface, context), "make_current");
    current = 1;
    LOAD(gles, glGetString); LOAD(gles, glGetStringi); LOAD(gles, glGetIntegerv); LOAD(gles, glGetError);
    LOAD(gles, glGenTextures); LOAD(gles, glBindTexture); LOAD(gles, glTexParameteri);
    LOAD(gles, glTexImage2D); LOAD(gles, glDeleteTextures); LOAD(gles, glGenFramebuffers);
    LOAD(gles, glBindFramebuffer); LOAD(gles, glFramebufferTexture2D);
    LOAD(gles, glCheckFramebufferStatus); LOAD(gles, glDeleteFramebuffers);
    LOAD(gles, glEnable); LOAD(gles, glDisable); LOAD(gles, glScissor);
    LOAD(gles, glClearColor); LOAD(gles, glClearDepthf); LOAD(gles, glClear);
    LOAD(gles, glPixelStorei); LOAD(gles, glReadPixels);
    gl_ready = 1;
    printf("gl_vendor=%.256s\ngl_renderer=%.256s\ngl_version=%.256s\nglsl_version=%.256s\n",
           text(glGetString(0x1f00)), text(glGetString(0x1f01)),
           text(glGetString(0x1f02)), text(glGetString(0x8b8c)));
    Int gl_major = 0, gl_minor = 0, extension_count = 0;
    glGetIntegerv(0x821b, &gl_major); glGetIntegerv(0x821c, &gl_minor);
    if (gl_major < 3) { printf("failure=context_is_not_es3\n"); goto cleanup; }
    printf("gl_context_version=%d.%d\n", gl_major, gl_minor);
    const UInt limits[] = {0x0d33, 0x84e8, 0x8cdf, 0x8824, 0x8d57, 0x8a30};
    const char *names[] = {"max_texture_size", "max_renderbuffer_size", "max_color_attachments",
                           "max_draw_buffers", "max_samples", "max_uniform_block_size"};
    for (size_t i = 0; i < sizeof(limits)/sizeof(*limits); ++i) {
        Int value = 0; glGetIntegerv(limits[i], &value); printf("%s=%d\n", names[i], value);
    }
    glGetIntegerv(0x821d, &extension_count);
    if (extension_count < 0 || extension_count > 512) { printf("failure=extension_count_bounds\n"); goto cleanup; }
    printf("gl_extensions_count=%d\ngl_extensions=", extension_count);
    for (Int i = 0; i < extension_count; ++i) printf("%.256s ", text(glGetStringi(0x1f03, (UInt)i)));
    printf("\n");
    UInt error = glGetError();
    if (error) { printf("failure=capability_queries gl_error=0x%x\n", error); goto cleanup; }
    glGenTextures(2, textures);
    for (int i = 0; i < 2; ++i) {
        glBindTexture(0x0de1, textures[i]);
        glTexParameteri(0x0de1, 0x2801, 0x2600); glTexParameteri(0x0de1, 0x2800, 0x2600);
        glTexImage2D(0x0de1, 0, i ? 0x8cac : 0x8058, 4, 4, 0,
                     i ? 0x1902 : 0x1908, i ? 0x1406 : 0x1401, NULL);
    }
    glGenFramebuffers(1, &fbo); glBindFramebuffer(0x8d40, fbo);
    glFramebufferTexture2D(0x8d40, 0x8ce0, 0x0de1, textures[0], 0);
    glFramebufferTexture2D(0x8d40, 0x8d00, 0x0de1, textures[1], 0);
    UInt status = glCheckFramebufferStatus(0x8d40);
    error = glGetError();
    printf("rgba8_depth32f_fbo_status=0x%x gl_error=0x%x\n", status, error);
    if (status != 0x8cd5 || error) goto cleanup;
    glDisable(0x0bd0); glDisable(0x0be2); glEnable(0x0c11); glClearDepthf(0.5f);
    const unsigned char colours[4][4] = {{255,0,0,255}, {0,255,0,255},
                                        {0,0,255,255}, {255,255,0,255}};
    for (int i = 0; i < 4; ++i) {
        glScissor((i % 2) * 2, (i / 2) * 2, 2, 2);
        glClearColor(colours[i][0]/255.0f, colours[i][1]/255.0f, colours[i][2]/255.0f, 1.0f);
        glClear(0x4000 | 0x0100);
    }
    glDisable(0x0c11); glPixelStorei(0x0d05, 1);
    unsigned char pixels[4*4*4] = {0};
    glReadPixels(0, 0, 4, 4, 0x1908, 0x1401, pixels);
    error = glGetError();
    if (error) { printf("failure=clear_readback gl_error=0x%x\n", error); goto cleanup; }
    for (int y = 0; y < 4; ++y) for (int x = 0; x < 4; ++x) {
        const unsigned char *expected = colours[(y/2)*2+x/2];
        const unsigned char *actual = pixels+(y*4+x)*4;
        if (memcmp(expected, actual, 4)) {
            printf("failure=asymmetric_pixels x=%d y=%d got=%u,%u,%u,%u\n",
                   x,y,actual[0],actual[1],actual[2],actual[3]); goto cleanup;
        }
    }
    printf("asymmetric_readback=PASS pixels=16 rgba_bytes=64\n");
    ok = 1;
cleanup:
    if (gl_ready && current) {
        if (fbo) glDeleteFramebuffers(1, &fbo);
        glDeleteTextures(2, textures);
        UInt cleanup_error = glGetError();
        printf("cleanup_gl_error=0x%x\n", cleanup_error);
        if (cleanup_error) ok = 0;
    }
    if (current) { UInt result = eglMakeCurrent(display, NULL, NULL, NULL);
        printf("cleanup_detach=%u\n", result); if (!result) ok = 0; }
    if (context) { UInt result = eglDestroyContext(display, context);
        printf("cleanup_context=%u\n", result); if (!result) ok = 0; }
    if (surface) { UInt result = eglDestroySurface(display, surface);
        printf("cleanup_surface=%u\n", result); if (!result) ok = 0; }
    if (initialized) { UInt result = eglTerminate(display);
        printf("cleanup_egl_terminate=%u\n", result); if (!result) ok = 0; }
    if (gles) dlclose(gles);
    if (egl) dlclose(egl);
    printf("result=%s\n", ok ? "PASS" : "FAIL");
    alarm(0);
    return ok ? 0 : 1;
}
