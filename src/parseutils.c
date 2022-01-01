#include "parseutils.h"

extern size_t esp_field_expect_zstring(struct esp_field *field, size_t off,
                                       char **zstring, size_t *len);

extern size_t esp_field_expect_constant_string(struct esp_field *field,
                                               size_t off,
                                               const char *expected_str,
                                               size_t expected_str_len,
                                               bool *got_expected);
