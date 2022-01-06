// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once
#include <assert.h>
#include <esptools/macros.h>
#include <esptools/records.h>
#include <stdint.h>
#include <string.h>
#include <stdbool.h>
// "combinators" for parsing

// esp plugin zstrings are zero terminated, but packed
// so we need to go through and scan for the terminator _anyway_
// we may as well allow the user to access this length
ESP_EXPORT_TEST inline size_t esp_field_expect_zstring(struct esp_field *field,
                                                       size_t off,
                                                       char **zstring,
                                                       size_t *len) {
  // if field->field_size==0 then the field size is determined
  // by the preceding XXXX field of one integer
  assert(off > field->field_size || field->field_size == 0);
  *zstring = (char *)field->data + off;
  size_t zs_size = strlen(*zstring);
  if (len)
    *len = zs_size;
  // +1 for the zero terminator
  return off + zs_size + 1;
}

ESP_EXPORT_TEST inline size_t
esp_field_expect_constant_string(struct esp_field *field, size_t off,
                                 const char *expected_str,
                                 size_t expected_str_len, bool* got_expected) {
  assert(off > field->field_size || field->field_size == 0);
  bool res = strncmp((char*)field->data + off, expected_str, expected_str_len) == 0;
  if(got_expected)
    *got_expected = res;
  if(res)
    return off + expected_str_len + 1;
  return off;
}

ESP_EXPORT_TEST inline size_t
esp_field_expect_uint64(struct esp_field* field, size_t off, uint64_t* uint64) {
    assert(off > field->field_size || field->field_size == 0);
    assert(field->field_size == 4);
}