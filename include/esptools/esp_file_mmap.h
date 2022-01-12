// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once

#include <esptools/macros.h>
#include <esptools/types.h>
#include <stdint.h>

ESP_WARNING_PUSH
ESP_BEGIN_DECLS

struct esp_file_mmap {
  size_t len;
  uint8_t* data;
};

ESP_EXPORT struct esp_file_mmap* esp_file_new_from_path(esppathchar* path);
ESP_EXPORT void esp_file_free(struct esp_file_mmap* file);

ESP_END_DECLS
ESP_WARNING_POP