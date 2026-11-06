/* Authored frontend contract, not SwanStation emulation or performance evidence. */
#define retro_init fixture_init
#define retro_load_game fixture_load_game
#define retro_serialize_size fixture_serialize_size
#define retro_get_memory_data fixture_memory_data
#define retro_get_memory_size fixture_memory_size
#include "../../../slot-retro/tests/fixtures/software.c"
#undef retro_init
#undef retro_load_game
#undef retro_serialize_size
#undef retro_get_memory_data
#undef retro_get_memory_size

static uint8_t card[131072];
static bool configured;
struct variable { const char *key; const char *value; };
void retro_init(void) {
    fixture_init();
    const char *expected[][2] = {
        {"swanstation_GPU_Renderer", "Software"},
        {"swanstation_GPU_ResolutionScale", "1"},
        {"swanstation_GPU_PGXPEnable", "false"},
        {"swanstation_MemoryCards_Card1Type", "Libretro"},
        {"swanstation_MemoryCards_Card2Type", "None"}
    };
    for (unsigned i = 0; i < sizeof(expected)/sizeof(expected[0]); i++) {
        struct variable v = {expected[i][0], NULL};
        if (!env(15, &v) || !v.value || strcmp(v.value, expected[i][1])) return;
    }
    const char *saves = NULL;
    configured = env(31, &saves) && saves && strstr(saves, "/PS/swanstation/");
}
bool retro_load_game(const struct game_info *info) {
    return configured && fixture_load_game(info);
}
size_t retro_serialize_size(void) { return 16; }
void *retro_get_memory_data(unsigned id) { return id == 0 ? card : NULL; }
size_t retro_get_memory_size(unsigned id) { return id == 0 ? sizeof(card) : 0; }
