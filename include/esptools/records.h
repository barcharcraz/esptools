// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once
#include <stdint.h>
#include <assert.h>
#include <esptools/macros.h>
ESP_WARNING_PUSH
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

struct esp_record_header {
    char type[4];
    // this one doesn't include this header
    uint32_t data_size;
    uint32_t flags;
    uint32_t formID;
    uint16_t timestamp;
    uint16_t vcs_info;
    uint16_t internal_version;
    uint16_t unknown;
};

_Static_assert(sizeof(struct esp_group_header) == 24, "esp_group_header is the wrong size");

struct esp_group {
    struct esp_group_header;
    uint8_t data[];
};

struct esp_record {
    struct esp_record_header;
    uint8_t data[];
};

struct esp_field {
    char type[4];
    // usually, a preceding XXXX (literal) field
    // can store larger amounts of data
    uint16_t field_size;
    uint8_t data[];
};

struct esp_HEDR_TES4 {
    float version;
    uint32_t nr_recs;
    uint32_t nxt_id;
};



ESP_END_DECLS
ESP_WARNING_POP