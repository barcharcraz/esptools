// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once
#include <esptools/macros.h>

ESP_WARNING_PUSH
ESP_BEGIN_DECLS

struct esp_record_track_value_direct {
  uint32_t num_refs;
  uint32_t** ref_locations;
};

ESP_END_DECLS
ESP_WARNING_POP