/* SPDX-License-Identifier: GPL-3.0-or-later
 * Opt-in HOST diagnostic, not Slot's transport implementation or a Swan runner.
 * Two surfaceless pbuffers; no window, swap, audio device or user files.
 * Build/run instructions and evidence limits: TRANSPORT-HOST.md.
 */
#define _POSIX_C_SOURCE 200809L
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#define SLOTS 3
#define WIDTH 1024
#define HEIGHT 512
#define REQUIRE(x, reason) do { if (!(x)) { \
    fprintf(stderr, "STOP reason=%s line=%d\n", reason, __LINE__); exit(2); \
} } while (0)
#define CHECK(x) REQUIRE(x, #x)

enum State { FREE, WRITING, READY, READING, RETIRING };
enum Fault {
    NONE, NO_PRODUCER_FENCE, NO_CONSUMER_FENCE, STALE_GENERATION,
    WRONG_ORIGIN, FENCE_TIMEOUT, OVERSIZE, PUBLISH_AFTER_LOSS, CAPTURE_AFTER_LOSS
};
struct Frame { uint64_t sequence, generation; int width, height; bool bottom_left; };
struct Slot { enum State state; GLuint texture, fbo; EGLSyncKHR fence; struct Frame frame; };
struct Session {
    EGLDisplay display;
    EGLSurface surfaces[2];
    EGLContext contexts[2];
    PFNEGLCREATESYNCKHRPROC create;
    PFNEGLCLIENTWAITSYNCKHRPROC wait;
    PFNEGLDESTROYSYNCKHRPROC destroy;
    PFNEGLSIGNALSYNCKHRPROC signal;
    pthread_mutex_t lock;
    pthread_cond_t changed;
    pthread_t worker;
    struct Slot slots[SLOTS];
    GLuint stable_texture, stable_fbo, reader;
    struct Frame requested;
    uint64_t generation, sequence, acquired_sequence;
    bool command, done, stop, published, lost;
    int reading;
    enum Fault fault;
    unsigned drops, publications, checks;
    atomic_uint fences_created, fences_destroyed;
};

static void tick(void) {
    struct timespec t = {.tv_nsec = 1000000};
    nanosleep(&t, NULL);
}

static EGLSyncKHR new_sync(struct Session *s, EGLenum type) {
    EGLSyncKHR f = s->create(s->display, type, NULL);
    CHECK(f != EGL_NO_SYNC_KHR);
    atomic_fetch_add(&s->fences_created, 1);
    return f;
}

static EGLSyncKHR fence(struct Session *s) {
    EGLSyncKHR f = new_sync(s, EGL_SYNC_FENCE_KHR);
    glFlush();
    return f;
}

static bool signalled(struct Session *s, EGLSyncKHR f) {
    REQUIRE(f != EGL_NO_SYNC_KHR, "missing-fence");
    EGLint result = s->wait(s->display, f, 0, 0);
    CHECK(result == EGL_CONDITION_SATISFIED_KHR || result == EGL_TIMEOUT_EXPIRED_KHR);
    return result == EGL_CONDITION_SATISFIED_KHR;
}

static void destroy_fence(struct Session *s, EGLSyncKHR f) {
    CHECK(f != EGL_NO_SYNC_KHR && s->destroy(s->display, f));
    atomic_fetch_add(&s->fences_destroyed, 1);
}

static void target(GLuint *texture, GLuint *fbo) {
    glGenTextures(1, texture);
    glBindTexture(GL_TEXTURE_2D, *texture);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, WIDTH, HEIGHT, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL);
    glGenFramebuffers(1, fbo);
    glBindFramebuffer(GL_FRAMEBUFFER, *fbo);
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, *texture, 0);
    CHECK(glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE);
}

/* Only the producer examines Retiring; only the consumer examines Ready.
 * GL/EGL operations are outside the CPU lock. Reading is never producer-owned.
 */
static void recycle(struct Session *s) {
    for (int i = 0; i < SLOTS; i++) {
        pthread_mutex_lock(&s->lock);
        enum State state = s->slots[i].state;
        EGLSyncKHR f = s->slots[i].fence;
        pthread_mutex_unlock(&s->lock);
        if (state != RETIRING || !signalled(s, f)) continue;
        destroy_fence(s, f);
        pthread_mutex_lock(&s->lock);
        s->slots[i].fence = EGL_NO_SYNC_KHR;
        s->slots[i].state = FREE;
        pthread_mutex_unlock(&s->lock);
    }
}

static bool valid_geometry(struct Frame f) {
    return f.width > 0 && f.height > 0 && f.width <= WIDTH && f.height <= HEIGHT;
}

/* Authored pattern: each sequence has a unique RG pair, with independent
 * left/right B and top/bottom G markers. Odd dimensions catch origin errors.
 */
static void render(struct Session *s, struct Frame f) {
    REQUIRE(valid_geometry(f), "unsupported-geometry");
    glBindFramebuffer(GL_FRAMEBUFFER, s->stable_fbo);
    glEnable(GL_SCISSOR_TEST);
    for (int y = 0; y < 2; y++) for (int x = 0; x < 2; x++) {
        int left = x ? f.width / 2 : 0;
        int right = x ? f.width : f.width / 2;
        int top = y ? f.height / 2 : 0;
        int bottom = y ? f.height : f.height / 2;
        glScissor(left, f.bottom_left ? f.height - bottom : top, right - left, bottom - top);
        glClearColor((f.sequence & 255) / 255.f,
            ((y ? 161 : 31) + ((f.sequence >> 8) & 63)) / 255.f,
            (x ? 223 : 17) / 255.f, 1.f);
        glClear(GL_COLOR_BUFFER_BIT);
    }
    glDisable(GL_SCISSOR_TEST);
}

static void publish(struct Session *s, struct Frame f) {
    recycle(s);
    render(s, f); /* A full pool skips publication, not the stand-in core step. */
    int slot = -1;
    pthread_mutex_lock(&s->lock);
    for (int i = 0; i < SLOTS; i++) if (s->slots[i].state == FREE) {
        slot = i;
        s->slots[i].state = WRITING;
        break;
    }
    pthread_mutex_unlock(&s->lock);
    s->published = slot >= 0;
    if (slot < 0) { s->drops++; return; }
    glBindFramebuffer(GL_READ_FRAMEBUFFER, s->stable_fbo);
    glBindFramebuffer(GL_DRAW_FRAMEBUFFER, s->slots[slot].fbo);
    glBlitFramebuffer(0, 0, f.width, f.height, 0, 0, f.width, f.height, GL_COLOR_BUFFER_BIT, GL_NEAREST);
    CHECK(glGetError() == GL_NO_ERROR);
    EGLSyncKHR ready = s->fault == NO_PRODUCER_FENCE ? EGL_NO_SYNC_KHR : fence(s);
    if (s->fault == FENCE_TIMEOUT) {
        destroy_fence(s, ready);
        ready = new_sync(s, EGL_SYNC_REUSABLE_KHR);
    }
    if (s->fault == STALE_GENERATION) f.generation--;
    if (s->fault == WRONG_ORIGIN) f.bottom_left = !f.bottom_left;
    pthread_mutex_lock(&s->lock);
    s->slots[slot].frame = f;
    s->slots[slot].fence = ready;
    s->slots[slot].state = READY;
    pthread_mutex_unlock(&s->lock);
    s->publications++;
}

static void *producer(void *arg) {
    struct Session *s = arg;
    CHECK(eglBindAPI(EGL_OPENGL_ES_API));
    CHECK(eglMakeCurrent(s->display, s->surfaces[1], s->surfaces[1], s->contexts[1]));
    if (s->generation == 1) printf("producer_actual=%s renderer=%s\n", glGetString(GL_VERSION), glGetString(GL_RENDERER));
    target(&s->stable_texture, &s->stable_fbo);
    for (int i = 0; i < SLOTS; i++) target(&s->slots[i].texture, &s->slots[i].fbo);
    pthread_mutex_lock(&s->lock);
    s->done = true;
    pthread_cond_signal(&s->changed);
    for (;;) {
        while (!s->command && !s->stop) pthread_cond_wait(&s->changed, &s->lock);
        if (s->stop) break;
        struct Frame f = s->requested;
        s->command = false;
        pthread_mutex_unlock(&s->lock);
        publish(s, f);
        pthread_mutex_lock(&s->lock);
        s->done = true;
        pthread_cond_signal(&s->changed);
    }
    bool lost = s->lost;
    pthread_mutex_unlock(&s->lock);
    if (!lost) {
        bool drained = false;
        for (int t = 0; t < 250; t++) {
            recycle(s);
            drained = true;
            for (int i = 0; i < SLOTS; i++) if (s->slots[i].state != FREE) drained = false;
            if (drained) break;
            tick();
        }
        CHECK(drained);
        for (int i = 0; i < SLOTS; i++) {
            glDeleteFramebuffers(1, &s->slots[i].fbo);
            glDeleteTextures(1, &s->slots[i].texture);
        }
        glDeleteFramebuffers(1, &s->stable_fbo);
        glDeleteTextures(1, &s->stable_texture);
        CHECK(glGetError() == GL_NO_ERROR);
    }
    /* Simulated loss deliberately skips GL calls/deletions. EGL share-group
     * destruction below owns final reclamation; this is not real driver loss.
     */
    CHECK(eglMakeCurrent(s->display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT));
    return NULL;
}

static void await_command(struct Session *s) {
    pthread_mutex_lock(&s->lock);
    while (!s->done) pthread_cond_wait(&s->changed, &s->lock);
    pthread_mutex_unlock(&s->lock);
}

static void submit(struct Session *s) {
    static const int sizes[4][2] = {{7, 5}, {320, 224}, {320, 448}, {3, 9}};
    REQUIRE(!s->lost, "invalid-session");
    uint64_t n = ++s->sequence;
    pthread_mutex_lock(&s->lock);
    CHECK(s->done && !s->command);
    s->requested = (struct Frame){n, s->generation, sizes[n % 4][0], sizes[n % 4][1], (n & 4) != 0};
    if (s->generation == 1 && n == 2) {
        s->requested.width = WIDTH;
        s->requested.height = HEIGHT;
    }
    if (s->fault == OVERSIZE) s->requested.width = WIDTH + 1;
    s->done = false;
    s->command = true;
    pthread_cond_signal(&s->changed);
    pthread_mutex_unlock(&s->lock);
}

static void release(struct Session *s) {
    if (s->reading < 0) return;
    CHECK(!s->lost);
    EGLSyncKHR consumed = s->fault == NO_CONSUMER_FENCE ? EGL_NO_SYNC_KHR : fence(s);
    pthread_mutex_lock(&s->lock);
    struct Slot *slot = &s->slots[s->reading];
    CHECK(slot->state == READING);
    slot->fence = consumed;
    slot->state = RETIRING;
    s->reading = -1;
    pthread_mutex_unlock(&s->lock);
}

static bool acquire(struct Session *s) {
    CHECK(!s->lost);
    struct Slot ready[SLOTS];
    bool complete[SLOTS] = {false};
    int newest = -1;
    pthread_mutex_lock(&s->lock);
    memcpy(ready, s->slots, sizeof ready);
    pthread_mutex_unlock(&s->lock);
    for (int i = 0; i < SLOTS; i++) if (ready[i].state == READY) {
        REQUIRE(ready[i].frame.generation == s->generation, "stale-generation");
        REQUIRE(valid_geometry(ready[i].frame), "unsupported-geometry");
        complete[i] = signalled(s, ready[i].fence);
        if (complete[i] && ready[i].frame.sequence > s->acquired_sequence &&
            (newest < 0 || ready[i].frame.sequence > ready[newest].frame.sequence)) newest = i;
    }
    if (newest >= 0) release(s);
    /* Late completions still retire, but can never rewind the current image. */
    for (int i = 0; i < SLOTS; i++) if (complete[i]) {
        destroy_fence(s, ready[i].fence);
        pthread_mutex_lock(&s->lock);
        s->slots[i].fence = EGL_NO_SYNC_KHR;
        s->slots[i].state = i == newest ? READING : FREE;
        pthread_mutex_unlock(&s->lock);
    }
    if (newest < 0) return false;
    s->reading = newest;
    s->acquired_sequence = ready[newest].frame.sequence;
    glBindTexture(GL_TEXTURE_2D, ready[newest].texture);
    return true;
}

static void acquire_tick(struct Session *s) {
    for (int t = 0; t < 250; t++) {
        if (acquire(s)) return;
        tick();
    }
    REQUIRE(false, "fence-timeout");
}

static void capture_check(struct Session *s) {
    REQUIRE(!s->lost && s->reading >= 0, "invalid-session");
    struct Slot slot = s->slots[s->reading];
    CHECK(slot.state == READING && slot.frame.generation == s->generation);
    struct Frame f = slot.frame;
    size_t length = (size_t)f.width * f.height * 4;
    unsigned char *pixels = malloc(length);
    CHECK(pixels);
    glBindFramebuffer(GL_FRAMEBUFFER, s->reader);
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, slot.texture, 0);
    CHECK(glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE);
    /* Capture owns pack alignment, rather than inheriting external padding. */
    GLint old_pack;
    glGetIntegerv(GL_PACK_ALIGNMENT, &old_pack);
    CHECK(old_pack == 8);
    glPixelStorei(GL_PACK_ALIGNMENT, 1);
    glReadPixels(0, 0, f.width, f.height, GL_RGBA, GL_UNSIGNED_BYTE, pixels);
    glPixelStorei(GL_PACK_ALIGNMENT, old_pack);
    CHECK(glGetError() == GL_NO_ERROR);
    for (int y = 0; y < f.height; y++) for (int x = 0; x < f.width; x++) {
        int row = f.bottom_left ? f.height - y - 1 : y;
        unsigned char *p = pixels + ((size_t)row * f.width + x) * 4;
        REQUIRE(p[0] == (f.sequence & 255), "frame-pattern");
        REQUIRE(p[1] == (y < f.height / 2 ? 31 : 161) + ((f.sequence >> 8) & 63), "frame-pattern");
        REQUIRE(p[2] == (x < f.width / 2 ? 17 : 223) && p[3] == 255, "frame-pattern");
    }
    free(pixels);
    s->checks++;
}

static void start(struct Session *s, unsigned generation, enum Fault fault) {
    memset(s, 0, sizeof *s);
    s->generation = generation;
    s->reading = -1;
    s->fault = fault;
    atomic_init(&s->fences_created, 0);
    atomic_init(&s->fences_destroyed, 0);
    CHECK(pthread_mutex_init(&s->lock, NULL) == 0);
    CHECK(pthread_cond_init(&s->changed, NULL) == 0);
    s->display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
    CHECK(eglInitialize(s->display, NULL, NULL));
    CHECK(eglBindAPI(EGL_OPENGL_ES_API));
    const char *extensions = eglQueryString(s->display, EGL_EXTENSIONS);
    CHECK(extensions && strstr(extensions, "EGL_KHR_fence_sync"));
    /* Reusable sync is test-only: deterministic pending-fence injection. */
    CHECK(strstr(extensions, "EGL_KHR_reusable_sync"));
    s->create = (PFNEGLCREATESYNCKHRPROC)eglGetProcAddress("eglCreateSyncKHR");
    s->wait = (PFNEGLCLIENTWAITSYNCKHRPROC)eglGetProcAddress("eglClientWaitSyncKHR");
    s->destroy = (PFNEGLDESTROYSYNCKHRPROC)eglGetProcAddress("eglDestroySyncKHR");
    s->signal = (PFNEGLSIGNALSYNCKHRPROC)eglGetProcAddress("eglSignalSyncKHR");
    CHECK(s->create && s->wait && s->destroy && s->signal);
    EGLint attrs[] = {EGL_SURFACE_TYPE, EGL_PBUFFER_BIT, EGL_RENDERABLE_TYPE,
        EGL_OPENGL_ES2_BIT | EGL_OPENGL_ES3_BIT_KHR, EGL_RED_SIZE, 8,
        EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_ALPHA_SIZE, 8, EGL_NONE};
    EGLConfig config;
    EGLint count;
    CHECK(eglChooseConfig(s->display, attrs, &config, 1, &count) && count == 1);
    EGLint size[] = {EGL_WIDTH, 64, EGL_HEIGHT, 64, EGL_NONE};
    for (int i = 0; i < 2; i++) {
        EGLint version[] = {EGL_CONTEXT_CLIENT_VERSION, i == 0 ? 2 : 3, EGL_NONE};
        s->surfaces[i] = eglCreatePbufferSurface(s->display, config, size);
        s->contexts[i] = eglCreateContext(s->display, config, i == 0 ? EGL_NO_CONTEXT : s->contexts[0], version);
        CHECK(s->surfaces[i] != EGL_NO_SURFACE && s->contexts[i] != EGL_NO_CONTEXT);
    }
    CHECK(eglMakeCurrent(s->display, s->surfaces[0], s->surfaces[0], s->contexts[0]));
    if (generation == 1) printf("consumer_requested=2 consumer_actual=%s\n", glGetString(GL_VERSION));
    glGenFramebuffers(1, &s->reader);
    glPixelStorei(GL_PACK_ALIGNMENT, 8);
    CHECK(pthread_create(&s->worker, NULL, producer, s) == 0);
    await_command(s);
}

static void close_session(struct Session *s, bool simulate_loss) {
    CHECK(!s->command && s->done);
    if (!simulate_loss) {
        /* Release all Ready ownership before join; producer cannot service UI. */
        bool pending = true;
        for (int t = 0; t < 250 && pending; t++) {
            acquire(s);
            pending = false;
            for (int i = 0; i < SLOTS; i++) if (s->slots[i].state == READY) pending = true;
            if (pending) tick();
        }
        CHECK(!pending);
        glBindFramebuffer(GL_FRAMEBUFFER, 0);
        glDeleteFramebuffers(1, &s->reader);
        glBindTexture(GL_TEXTURE_2D, 0);
        release(s);
    }
    pthread_mutex_lock(&s->lock);
    s->lost = simulate_loss;
    if (simulate_loss) s->generation++;
    s->stop = true;
    pthread_cond_signal(&s->changed);
    pthread_mutex_unlock(&s->lock);
    CHECK(pthread_join(s->worker, NULL) == 0);
    if (simulate_loss) {
        /* No more capture/publish/relaunch: all old descriptors are invalid. */
        for (int i = 0; i < SLOTS; i++) {
            if (s->slots[i].state != FREE) CHECK(s->slots[i].frame.generation != s->generation);
            if (s->slots[i].fence != EGL_NO_SYNC_KHR) destroy_fence(s, s->slots[i].fence);
        }
    }
    CHECK(atomic_load(&s->fences_created) == atomic_load(&s->fences_destroyed));
    CHECK(eglMakeCurrent(s->display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT));
    for (int i = 1; i >= 0; i--) {
        CHECK(eglDestroyContext(s->display, s->contexts[i]));
        CHECK(eglDestroySurface(s->display, s->surfaces[i]));
    }
    CHECK(eglTerminate(s->display));
    CHECK(pthread_cond_destroy(&s->changed) == 0);
    CHECK(pthread_mutex_destroy(&s->lock) == 0);
}

static void pressure(struct Session *s) {
    uint64_t held = s->slots[s->reading].frame.sequence;
    for (int i = 0; i < 2; i++) {
        submit(s);
        capture_check(s); /* Consumer and producer operate on different threads. */
        await_command(s);
        CHECK(s->published);
    }
    unsigned drops = s->drops;
    for (int i = 0; i < 32; i++) {
        submit(s);
        capture_check(s);
        await_command(s);
        CHECK(!s->published && s->slots[s->reading].frame.sequence == held);
    }
    CHECK(s->drops == drops + 32);
    acquire_tick(s);
    capture_check(s);
    CHECK(s->slots[s->reading].frame.sequence == held + 2);
}

/* The harness replaces an already-completed GPU fence with a genuine but
 * deliberately unsignalled EGL reusable sync. No unfinished GPU dependency
 * is discarded. This tests polling/ownership policy, not a driver GPU stall.
 */
static EGLSyncKHR hold_completion(struct Session *s, int i) {
    EGLSyncKHR old = s->slots[i].fence;
    bool completed = false;
    for (int t = 0; t < 250; t++) {
        if (signalled(s, old)) { completed = true; break; }
        tick();
    }
    CHECK(completed);
    destroy_fence(s, old);
    EGLSyncKHR held = new_sync(s, EGL_SYNC_REUSABLE_KHR);
    pthread_mutex_lock(&s->lock);
    s->slots[i].fence = held;
    pthread_mutex_unlock(&s->lock);
    return held;
}

static void pending_checks(struct Session *s) {
    submit(s);
    await_command(s);
    CHECK(s->published);
    int ready = -1;
    for (int i = 0; i < SLOTS; i++) if (s->slots[i].state == READY) ready = i;
    CHECK(ready >= 0);
    EGLSyncKHR held = hold_completion(s, ready);
    uint64_t old = s->slots[s->reading].frame.sequence;
    for (int t = 0; t < 3; t++) {
        CHECK(!acquire(s));
        CHECK(s->slots[s->reading].frame.sequence == old);
        tick();
    }
    capture_check(s);
    CHECK(s->signal(s->display, held, EGL_SIGNALED_KHR));
    acquire_tick(s);
    capture_check(s);

    submit(s);
    await_command(s);
    CHECK(s->published);
    int delayed = -1;
    for (int i = 0; i < SLOTS; i++) if (s->slots[i].state == READY) delayed = i;
    CHECK(delayed >= 0);
    held = hold_completion(s, delayed);
    submit(s);
    await_command(s);
    CHECK(s->published);
    acquire_tick(s);
    uint64_t newer = s->slots[s->reading].frame.sequence;
    CHECK(newer == s->sequence);
    CHECK(s->signal(s->display, held, EGL_SIGNALED_KHR));
    REQUIRE(!acquire(s), "late-frame-must-not-rewind");
    CHECK(s->slots[s->reading].frame.sequence == newer);
    capture_check(s);

    int retiring = s->reading;
    release(s);
    held = hold_completion(s, retiring);
    for (int t = 0; t < 2; t++) {
        submit(s);
        await_command(s);
        CHECK(s->published && s->slots[retiring].state == RETIRING);
    }
    submit(s);
    await_command(s);
    CHECK(!s->published && s->slots[retiring].state == RETIRING);
    CHECK(s->signal(s->display, held, EGL_SIGNALED_KHR));
    submit(s);
    await_command(s);
    CHECK(s->published);
    acquire_tick(s);
    capture_check(s);
}

int main(int argc, char **argv) {
    alarm(180);
    bool quick = argc == 2 && strcmp(argv[1], "--quick") == 0;
    enum Fault fault = NONE;
    if (argc == 3 && strcmp(argv[1], "--fault") == 0) {
        if (!strcmp(argv[2], "producer-fence")) fault = NO_PRODUCER_FENCE;
        else if (!strcmp(argv[2], "consumer-fence")) fault = NO_CONSUMER_FENCE;
        else if (!strcmp(argv[2], "generation")) fault = STALE_GENERATION;
        else if (!strcmp(argv[2], "origin")) fault = WRONG_ORIGIN;
        else if (!strcmp(argv[2], "timeout")) fault = FENCE_TIMEOUT;
        else if (!strcmp(argv[2], "oversize")) fault = OVERSIZE;
        else if (!strcmp(argv[2], "publish-after-loss")) fault = PUBLISH_AFTER_LOSS;
        else if (!strcmp(argv[2], "capture-after-loss")) fault = CAPTURE_AFTER_LOSS;
        else CHECK(!"unknown fault");
        quick = true;
    } else CHECK(argc == 1 || quick);
    CHECK(!valid_geometry((struct Frame){.width = 1025, .height = 1}));
    CHECK(!valid_geometry((struct Frame){.width = 1, .height = 513}));
    CHECK(!valid_geometry((struct Frame){.width = 0, .height = 224}));
    unsigned total = 0, drops = 0, losses = 0, publications = 0, syncs = 0;
    int sessions = quick ? 1 : 101;
    for (int lifecycle = 1; lifecycle <= sessions; lifecycle++) {
        struct Session s;
        start(&s, lifecycle, fault);
        int frames = quick ? 1 : lifecycle == 1 ? 10000 : 16;
        uint64_t previous = 0;
        for (int i = 0; i < frames; i++) {
            submit(&s);
            await_command(&s);
            if (!s.published) { tick(); i--; continue; }
            acquire_tick(&s);
            CHECK(s.slots[s.reading].frame.sequence > previous);
            previous = s.slots[s.reading].frame.sequence;
            capture_check(&s);
        }
        pressure(&s);
        pending_checks(&s);
        /* Close at full capacity: one held lease and two Ready publications. */
        for (int i = 0; i < 2; i++) {
            submit(&s);
            await_command(&s);
            CHECK(s.published);
        }
        bool loss = lifecycle % 10 == 0 || fault == PUBLISH_AFTER_LOSS || fault == CAPTURE_AFTER_LOSS;
        close_session(&s, loss);
        if (fault == PUBLISH_AFTER_LOSS) submit(&s);
        if (fault == CAPTURE_AFTER_LOSS) capture_check(&s);
        total += s.checks;
        drops += s.drops;
        losses += loss;
        publications += s.publications;
        syncs += atomic_load(&s.fences_destroyed);
    }
    printf("PASS sessions=%d captures=%u publications=%u publication_drops=%u simulated_losses=%u balanced_syncs=%u slots=%d stable_fbo=1024x512\n",
        sessions, total, publications, drops, losses, syncs, SLOTS);
    puts("SCOPE=surfaceless synthetic correctness; no Swan/window/speed acceptance");
    return 0;
}
