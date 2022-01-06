// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once
#include <assert.h>
#include <esptools/macros.h>
#include <stdint.h>
ESP_WARNING_PUSH
ESP_BEGIN_DECLS

struct esp_group_header {
  char type[4];
  // includes this header
  uint32_t group_size;
  uint8_t label[4];
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

struct esp_field_header {
  char type[4];
  // usually, a preceding XXXX (literal) field
  // can store larger amounts of data
  uint16_t field_size;
};

_Static_assert(sizeof(struct esp_group_header) == 24,
               "esp_group_header is the wrong size");

struct esp_group {
  struct esp_group_header;
  uint8_t data[];
};

struct esp_record {
  struct esp_record_header;
  uint8_t data[];
};

struct esp_field {
  struct esp_field_header;
  uint8_t data[];
};

ESP_EXPORT struct esp_field*
esp_record_first_field(struct esp_record* rec);

/**
 *  esp_record_next_field
 *
 * @rec: (transfer none): record
 * @prv_field: (transfer none, nullable): previous (or "current") field
 * @field_size: (nullable, inout): the true size of the returned field. in most
 *    cases this is equal to field->field_size, however if prv_field is an XXXX
 *    field then it's set to the data of that field when calling this function,
 *    if this parameter is not null it should be set to the "true size" of
 *    prv_field.
 *
 *    It is an error to call this function with field_size set to null if
 *    prev_field is of type XXXX, or, if the "true size" of prv_field was
 *    determined from an XXXX record
 *
 *    If prv_field is null then get the first field, and, if field_size is
 *    non-null, write the size (which will always be equal to field->field_size)
 *    into the integer pointed to by field_size.
 *
 * returns: (transfer none): a pointer to the next field, or null if
 *  there are no more fields
 */
ESP_EXPORT struct esp_field *
esp_record_next_field(struct esp_record *rec,
                      struct esp_field const *prv_field,
                      uint32_t* field_size);



ESP_EXPORT struct esp_field*
esp_record_field_bytype(struct esp_record* rec,
  const char type[4]);

ESP_END_DECLS
ESP_WARNING_POP