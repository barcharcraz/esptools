// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#include <esptools/esptools.h>
#include <parseutils.h>
#include <assert.h>

int main(void) {
    int tint = 'TES4';
    const char* tz = "TES4";
    int tint2 = *(uint32_t*)tz;
    assert(tint == tint2);
    return 0;
}