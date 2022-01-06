#include <assert.h>
#include <esptools/records.h>
#include <stdint.h>
#include <string.h>

struct esp_field *esp_record_first_field(struct esp_record *rec) {
  if (rec->data_size == 0)
    return NULL;
  return (struct esp_field *)rec->data;
}

struct esp_field *esp_record_next_field(struct esp_record *rec,
                                        struct esp_field const *prv_field,
                                        uint32_t *field_size) {
  struct esp_field *result = 0;
  if (!prv_field) {
    // get the first field
    result = esp_record_first_field(rec);
    if (result && field_size)
      *field_size = result->field_size;
    return result;
  }
  size_t true_size_of_prv = 0;
  if (field_size) {
    assert(prv_field->field_size == *field_size);
    true_size_of_prv = *field_size;
  } else {
    true_size_of_prv = prv_field->field_size;
  }
  size_t prv_field_offset = (uint8_t *)rec - (uint8_t *)prv_field;
  if ((prv_field_offset + true_size_of_prv) >=
      (rec->data_size + sizeof(struct esp_record_header))) {
    return NULL;
  }
  result =
      (struct esp_field *)(rec->data + prv_field_offset + true_size_of_prv);
  if (strncmp(prv_field->type, "XXXX", 4) == 0) {
    *field_size = *(uint32_t *)prv_field->data;
  } else {
    *field_size = prv_field->field_size;
  }
  return result;
}

struct esp_field *esp_record_field_bytype(struct esp_record *rec,
                                          const char type[4]) {
  uint32_t size = 0;
  struct esp_field *field = esp_record_next_field(rec, NULL, &size);
  while (field && memcmp(field->type, type, 4) != 0)
    field = esp_record_next_field(rec, field, &size);
  return field;
}