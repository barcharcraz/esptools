#pragma once
#include <esptools/macros.h>
#include <esptools/types.h>
#include <stdbool.h>

ESP_WARNING_PUSH
ESP_BEGIN_DECLS

struct esp_compact_persistence_data;

struct esp_compact_options {
    struct esp_zs_array_view load_order;
    struct esp_compact_persistence_data* persist;
    esppathchar* file_to_compact;
    esppathchar* backup_file;
};



ESP_END_DECLS
ESP_WARNING_POP