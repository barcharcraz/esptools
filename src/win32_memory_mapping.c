#define _WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include "memory_mapping.h"
#include <Windows.h>
#include <memoryapi.h>
#include <stddef.h>
#include <stdlib.h>

void *esp_map_file_ro(size_t len, espnativefd fd) {
  HANDLE h = CreateFileMappingW(fd, NULL, PAGE_READONLY,
                                (DWORD)((len >> 32) & 0xffffffff),
                                (DWORD)(len & 0xffffffff), NULL);
  if (!h) {
    return NULL;
  }

  void *addr = MapViewOfFile(h, FILE_MAP_READ, 0, 0, 0);

  // resources must be freed, should structurally always succeed
  if (!CloseHandle(h))
    abort();

  return addr;
}

int esp_unmap_file(void *addr, size_t len) {
  if (!UnmapViewOfFile(addr)) {
    return -1;
  }
  return 0;
}

struct esp_map_file_byname_result
esp_map_file_ro_byname(esppathchar *filepath) {
  struct esp_map_file_byname_result result = {0};
  HANDLE fileH = CreateFileW(filepath, GENERIC_READ, 0, NULL,
                                    OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
  if (fileH == INVALID_HANDLE_VALUE) {
    goto err_exit;
  }
  LARGE_INTEGER file_size;
  if(!GetFileSizeEx(fileH, &file_size)) {
    goto err_file;
  }
  result.len = file_size.QuadPart;
  result.addr = esp_map_file_ro(result.len, fileH);

err_file:
  if(!CloseHandle(fileH)) {
    abort();
  }
err_exit:
  return result;

}

int esp_unmap_file_byname(struct esp_map_file_byname_result to_unmap) {
  return esp_unmap_file(to_unmap.addr, to_unmap.len);
}