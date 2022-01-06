#include "memory_mapping.h"
#include "testconfig.h"
#include <assert.h>
#include <esptools/macros.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
  struct esp_map_file_byname_result mapping =
      esp_map_file_ro_byname(ESP_PATH_LITERAL(TEST_DATA_PATH) "/empty.esm");
  assert(strncmp(mapping.addr, "TES4", 4) == 0);
  assert(esp_unmap_file_byname(mapping) == 0);

  return EXIT_SUCCESS;
}