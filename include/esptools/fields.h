// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once
#include <esptools/records.h>

#define ESP_XMACRO_TES4_FIELDS \
RX(HEDR, struct esp_TES4_HEDR) \
OX(CNAM, char*) \
OX(SNAM, char*) \
XSG(RX(MAST, char*), RX(DATA, uint64_t)) \
OX(ONAM, uint32_t*) \
RX(INTV, uint32_t) \
OX(INCC, uint32_t)

#define ESP_XMACRO_FORMID_FIELDS \
X(ACHR, VMAD) \
X(ACHR, NAME) \
X(ACHR, XEZN) \
X(ACHR, INAM) \
X(ACHR, PDTO) \
X(ACHR, XAPR) \
X(ACHR, XLRT) \
X(ACHR, XHOR) \
X(ACHR, XESP) \
X(ACHR, XOWN) \
X(ACHR, XLCN) \
X(ACHR, XLKR) \
X(ACHR, XLRL) \
/* from ACTI */ \
X(ACTI, MODL) /* only the MODS subfield */ \
X(ACTI, DEST) /* the DSTD and DMDS subfield */ \
X(ACTI, KWDA) \
X(ACTI, SNAM) \
X(ACTI, VNAM) \
X(ACTI, WNAM) \
X(ACTI, KNAM)



#define ESP_XMACRO_FIELD(recname, fieldname, fieldtype) \
size_t esp_##recname##_##fieldname(struct esp_field const* recname, size_t off, fieldtype* fieldname)

#define X(fname, tname) ESP_XMACRO_FIELD(TES4, fname, tname);
// definitions are emitted here
ESP_XMACRO_TES4_FIELDS
#undef X
