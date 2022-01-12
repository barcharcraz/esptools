#include <esptools/esp_file_mmap.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "memory_mapping.h"



static void esp_file_init_from_path(struct esp_file_mmap* file, esppathchar* path) {
  struct esp_map_file_byname_result res = esp_map_file_ro_byname(path);
  file->data = res.addr;
  file->len = res.len;
}

static void esp_file_destroy(struct esp_file_mmap* file) {
  if(esp_unmap_file(file->data, file->len) == -1) {
    fputs("Could not unmap file\n", stderr);
    abort();
  }
  memset(file, 0, sizeof(struct esp_file_mmap));
}

struct esp_file_mmap* esp_file_new_from_path(esppathchar* path) {
  struct esp_file_mmap* result = malloc(sizeof(struct esp_file_mmap));
  memset(result, 0, sizeof(struct esp_file_mmap));
  esp_file_init_from_path(result, path);
  return result;
}

void esp_file_free(struct esp_file_mmap* file) {
  esp_file_destroy(file);
  free(file);
}