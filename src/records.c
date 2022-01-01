#include <assert.h>
#include <stdint.h>
#include <esptools/records.h>
#include <string.h>
struct esp_field *esp_record_next_field(struct esp_record *rec,
                                        struct esp_field const *prv_field,
                                        uint32_t *field_size) {
  size_t true_size_of_prv = 0;
  if (field_size) {
    assert(prv_field->field_size == *field_size);
    true_size_of_prv = *field_size;
  } else {
    true_size_of_prv = prv_field->field_size;
  }
  size_t prv_field_offset = (uint8_t*)rec - (uint8_t*)prv_field;
  if ((prv_field_offset + true_size_of_prv) >=
      (rec->data_size + sizeof(struct esp_record_header))) 
  {
    return NULL;
  }
  struct esp_field* result = (struct esp_field*)(rec->data + prv_field_offset + true_size_of_prv);
  if(strncmp(prv_field->type, "XXXX", 4) == 0) {
    *field_size = *(uint32_t*)prv_field->data;
  } else {
    *field_size = prv_field->field_size;
  }
  return result;

}