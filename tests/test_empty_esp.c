#include "memory_mapping.h"
#include "testconfig.h"
#include <esptools/esptools.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

void test_header(struct esp_record *rcd) {
  assert(strncmp(rcd->type, "TES4", 4) == 0);
  assert(rcd->data_size == 52);
  assert(rcd->flags == 1);
  assert(rcd->formID == 0);
  assert(rcd->timestamp == 0);
  assert(rcd->vcs_info == 0);
  assert(rcd->internal_version == 44);
  assert(rcd->unknown == 0);

  struct esp_field *tes4_field = (struct esp_field *)rcd->data;
  assert(strncmp(tes4_field->type, "HEDR", 4) == 0);
  assert(tes4_field->field_size == 12);
}

void test_fields(struct esp_record* rcd) {
  struct esp_field* field = 0;
  uint32_t field_size = 0;
  field = esp_record_next_field(rcd, field, &field_size);
  assert(memcmp(field->type, "HEDR", 4) == 0);
  field = esp_record_next_field(rcd, field, &field_size);
  assert(memcmp(field->type, "CNAM", 4) == 0);
  field = esp_record_next_field(rcd, field, &field_size);
  assert(memcmp(field->type, "INTV", 4) == 0);
  field = esp_record_next_field(rcd, field, &field_size);
  assert(memcmp(field->type, "INCC", 4) == 0);
  field = esp_record_next_field(rcd, field, &field_size);
  assert(field == NULL);
}

void test_fields_bytype(struct esp_record* rcd) {
  struct esp_field* field = esp_record_field_bytype(rcd, "INTV");
  assert(field);
  assert(memcmp(field->type, "INTV", 4) == 0);
}

int main(void) {
  struct esp_map_file_byname_result mapping =
      esp_map_file_ro_byname(ESP_PATH_LITERAL(TEST_DATA_PATH) "/empty.esm");
  assert(mapping.addr);
  struct esp_record *rcd = mapping.addr;
  test_header(rcd);
  test_fields(rcd);
  test_fields_bytype(rcd);


  return EXIT_SUCCESS;
}