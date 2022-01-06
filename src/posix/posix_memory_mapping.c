#include "memory_mapping.h"
#include <esptools/types.h>
#include <fcntl.h>
#include <stddef.h>
#include <sys/stat.h>
#include <stdlib.h>
#include <sys/mman.h>
void *esp_map_file_ro(size_t len, espnativefd fd) {
  void *addr = mmap(NULL, len, PROT_READ, MAP_PRIVATE, fd, 0);
  if (addr == MAP_FAILED) {
    return NULL;
  }
  return addr;
}
int esp_unmap_file(void *addr, size_t len) { return munmap(addr, len); }

struct esp_map_file_byname_result
esp_map_file_ro_byname(esppathchar *filepath) {
  struct esp_map_file_byname_result result = {0};
  int fd = open(filepath, O_CLOEXEC | O_RDONLY);
  if (fd == -1) {
    goto err_exit;
  }
  struct stat file_stat = {0};
  if (fstat(fd, &file_stat) == -1) {
    goto err_file;
  }
  result.len = file_stat.st_size;
  result.addr = esp_map_file_ro(result.len, fd);
err_file:
  if (close(fd) == -1) {
    abort();
  }
err_exit:
  return result;
}