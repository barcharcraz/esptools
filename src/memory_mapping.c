#if __has_include(<unistd.h>)
#include <unistd.h>
#endif

#ifdef _WIN32
#include "win32/win32_memory_mapping.c"
#elif defined(_POSIX_VERSION)
#include "posix/posix_memory_mapping.c"
#endif

int esp_unmap_file_byname(struct esp_map_file_byname_result to_unmap) {
  return esp_unmap_file(to_unmap.addr, to_unmap.len);
}