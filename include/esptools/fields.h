#pragma once
#include <esptools/records.h>

#define ESP_XMACRO_TES4_FIELDS \
X(HEDR, struct esp_TES4_HEDR) \
X(CNAM, char*) \
X(SNAM, char*) \
X(MAST, char*) \
X(DATA, uint64_t*) \
X(ONAM, uint32_t**) \
X(INTV, uint32_t*) \
X(INCC, uint32_t*)

#define ESP_XMACRO_FIELD(recname, fieldname, fieldtype) \
size_t esp_##recname##_##fieldname(struct esp_field const* recname, size_t off, fieldtype* fieldname)

#define X(fname, tname) ESP_XMACRO_FIELD(TES4, fname, tname);
// definitions are emitted here
ESP_XMACRO_TES4_FIELDS
#undef X
