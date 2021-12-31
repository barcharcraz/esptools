#include "parseutils.h"

extern size_t esp_field_expect_zstring(struct esp_field *field, size_t off,
                                       char **zstring, size_t *len);