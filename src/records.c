#include <assert.h>
#include <esptools/records.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct esp_field *esp_record_first_field(struct esp_record *rec) {
  if (rec->data_size == 0)
    return NULL;
  return (struct esp_field *)rec->data;
}

ESP_EXPORT struct esp_group *esp_file_first_group(struct esp_file_mmap *file) {
  struct esp_record_header *tes4 = (struct esp_record_header *)file->data;
  ESP_CHECK(memcmp(tes4->type, "TES4", 4) == 0,
            "Malformed file: No TES4 record");
  size_t first_grp_offset = sizeof(struct esp_record_header) + tes4->data_size;
  ESP_CHECK(first_grp_offset + sizeof(struct esp_group_header) > file->len,
            "Malformed file: truncated group");
  struct esp_group *result =
      (struct esp_group *)(file->data + first_grp_offset);
  ESP_CHECK(memcmp(result->type, "GRUP", 4) == 0,
            "Malformed file: First group is not a group");
  ESP_CHECK(result->group_size + first_grp_offset <= file->len,
            "Malformed file: truncated group");
  return result;
}

struct esp_group *esp_file_next_group(struct esp_file_mmap *file,
                                      struct esp_group const *prv_group) {
  assert(memcmp(prv_group->type, "GRUP", 4) == 0);
  assert(file < prv_group);
  size_t group_offset = (uint8_t *)prv_group - (uint8_t)file;
  size_t new_group_offset = group_offset + prv_group->group_size;
  if (new_group_offset == file->len) {
    // last group
    return NULL;
  }
  ESP_CHECK(new_group_offset + sizeof(struct esp_group_header) <= file->len,
            "Malformed file, truncated group");
  struct esp_group *result =
      (struct esp_group *)(file->data + new_group_offset);
  ESP_CHECK(new_group_offset + result->group_size <= file->len,
            "Malformed file: truncated group");
  return result;
}

struct esp_group *esp_file_next_named_group(struct esp_file_mmap *file,
                                            struct esp_group const *prv_group,
                                            const char group_name[4]) {
  assert(memcmp(prv_group->type, "GRUP", 4) == 0);
  struct 
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
  // offset into rec->data (not from the start of rec)

  size_t prv_field_offset =
      (uint8_t *)prv_field - (uint8_t *)rec - sizeof(struct esp_record_header);
  size_t next_field_offset =
      prv_field_offset + true_size_of_prv + sizeof(struct esp_field_header);
  if (next_field_offset >= rec->data_size) {
    return NULL;
  }

  result = (struct esp_field *)(rec->data + next_field_offset);
  if (memcmp(prv_field->type, "XXXX", 4) == 0) {
    *field_size = *(uint32_t *)prv_field->data;
  } else {
    *field_size = result->field_size;
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