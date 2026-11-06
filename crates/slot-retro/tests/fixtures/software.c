/* Authored libretro contract fixture. No ROM, BIOS or emulator source. */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

struct game_info { const char *path; const void *data; size_t size; const char *meta; };
struct system_info { const char *name, *version, *extensions; bool fullpath, block_extract; };
struct geometry { unsigned width, height, max_width, max_height; float aspect; };
struct timing { double fps, rate; };
struct av_info { struct geometry geometry; struct timing timing; };
static void (*video)(const void *, unsigned, unsigned, size_t);
static int16_t (*input)(unsigned, unsigned, unsigned, unsigned);
static bool (*env)(unsigned, void *);
static unsigned controller;
static uint32_t pixels[448][324];
static uint16_t small[128][132];
static unsigned disc;
static bool ejected;
static bool set_eject(bool value) { ejected = value; return true; }
static bool get_eject(void) { return ejected; }
static unsigned get_disc(void) { return disc; }
static unsigned get_count(void) { return 3; }
static bool set_disc(unsigned value) {
    if (!ejected || value >= 2) return false;
    disc = value;
    return true;
}
struct disks {
    bool (*set_eject)(bool);
    bool (*get_eject)(void);
    unsigned (*get_index)(void);
    bool (*set_index)(unsigned);
    unsigned (*get_count)(void);
    bool (*replace)(unsigned, const struct game_info *);
    bool (*add)(void);
};

void retro_init(void) {
    struct disks d = {set_eject, get_eject, get_disc, set_disc, get_count, NULL, NULL};
    env(13, &d);
}
void retro_deinit(void) {}
unsigned retro_api_version(void) { return 1; }
void retro_get_system_info(struct system_info *i) {
    *i = (struct system_info){"Slot contract fixture", "1", "cue", true, true};
}
void retro_get_system_av_info(struct av_info *i) {
    *i = (struct av_info){{320, 224, 320, 448, 4.f/3.f}, {50, 44100}};
}
void retro_set_environment(bool (*cb)(unsigned, void *)) { env = cb; }
void retro_set_video_refresh(void (*cb)(const void *, unsigned, unsigned, size_t)) { video = cb; }
void retro_set_audio_sample(void (*cb)(int16_t, int16_t)) { (void)cb; }
void retro_set_audio_sample_batch(size_t (*cb)(const int16_t *, size_t)) { (void)cb; }
void retro_set_input_poll(void (*cb)(void)) { (void)cb; }
void retro_set_input_state(int16_t (*cb)(unsigned, unsigned, unsigned, unsigned)) { input = cb; }
void retro_set_controller_port_device(unsigned port, unsigned device) {
    if (port == 0) controller = device;
}
bool retro_load_game(const struct game_info *i) {
    unsigned format = 1;
    env(10, &format);
    if (i && i->path && strstr(i->path, "advertised")) {
        struct description { const char *name; unsigned id; };
        struct controller_info { const struct description *types; unsigned count; };
        static const struct description types[] = {
            {"Analog Joystick", 517}, {"Analog Controller (DualShock)", 261}, {"Digital", 1}
        };
        static const struct controller_info ports[] = {{types, 3}, {NULL, 0}};
        env(35, (void *)ports);
    }
    return i && i->path && !i->data && i->size == 0;
}
void retro_unload_game(void) {}
void retro_reset(void) {}
void retro_run(void) {
    unsigned buttons = (uint16_t)input(0, 1, 0, 256);
    if (buttons & 8) { video(NULL, 320, 224, sizeof(pixels[0])); return; }
    if (buttons & 1) { video(pixels, 320, 224, 2); return; }
    if (buttons & 4) { video(pixels, 4096, 224, sizeof(pixels[0])); return; }
    if (buttons & 1024) {
        unsigned format = 2;
        struct geometry g = {128, 128, 128, 128, 1};
        env(10, &format);
        env(37, &g);
        memset(small, 0, sizeof(small));
        small[0][0] = 0xf800;
        small[127][127] = 0x07e0;
        video(small, 128, 128, sizeof(small[0]));
        return;
    }
    unsigned format = 1;
    env(10, &format);
    memset(pixels, 0, sizeof(pixels));
    pixels[0][0] = 0x00112233;
    pixels[223][319] = 0x00445566;
    pixels[0][1] = controller;
    pixels[0][2] = (uint16_t)input(0, 5, 0, 0);
    pixels[0][3] = (uint16_t)input(0, 5, 0, 1);
    pixels[0][4] = (uint16_t)input(0, 5, 1, 0);
    pixels[0][5] = (uint16_t)input(0, 5, 1, 1);
    pixels[0][6] = (uint16_t)input(0, 1, 0, 256);
    pixels[0][7] = disc | (ejected << 16);
    if (buttons & 256) {
        struct av_info av = {{320, 448, 320, 448, 4.f/3.f}, {60, 48000}};
        env(32, &av);
        pixels[447][319] = 0x00778899;
        video(pixels, 320, 448, sizeof(pixels[0]));
    } else {
        video(pixels, 320, 224, sizeof(pixels[0]));
    }
}
size_t retro_serialize_size(void) { return 0; }
bool retro_serialize(void *p, size_t n) { (void)p; (void)n; return false; }
bool retro_unserialize(const void *p, size_t n) { (void)p; (void)n; return false; }
void *retro_get_memory_data(unsigned id) { (void)id; return NULL; }
size_t retro_get_memory_size(unsigned id) { (void)id; return 0; }
