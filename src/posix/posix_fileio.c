
#include "fileio.h"
#include <fcntl.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>


// basic posix copy file that
// checks for errors, but isn't that
// graceful in handling them.
bool esp_copy_file(esppathchar *src, esppathchar *dst) {
  bool result = false;
  int srcfd = open(src, O_RDONLY | O_CLOEXEC);
  if (srcfd == -1) {
    goto err1;
  }
  int dstfd = open(dst, O_WRONLY | O_CLOEXEC | O_CREAT | O_EXCL);
  if (dstfd == -1) {
    goto err2;
  }
  uint8_t buf[4096];
  ssize_t sz = read(srcfd, buf, sizeof(buf));
  while(sz != 0) {
    if (write(dstfd, buf, sz) != sz) {
      goto err3;
    }
    sz = read(srcfd, buf, sizeof(buf));
  }
  result = true;
err3:
  if(close(dstfd) == -1) {
    abort();
  }
err2:
  if (close(srcfd) == -1) {
    abort();
  }
err1:
  return result;
}