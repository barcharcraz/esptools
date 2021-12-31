#pragma once
#include <stdint.h>
#include <assert.h>
#include <esptools/macros.h>

ESP_BEGIN_DECLS

struct esp_group_header {
    char type[4];
    // includes this header
    uint32_t group_size;
    uint8_t label;
    int32_t group_type;
    uint16_t timestamp;
    uint16_t vcs_info;
    uint32_t unknown;
};

_Static_assert(sizeof(struct esp_group_header) == 24, "esp_group_header is the wrong size");

struct esp_group {
    struct esp_group_header;
    uint8_t data[];
};

ESP_END_DECLS