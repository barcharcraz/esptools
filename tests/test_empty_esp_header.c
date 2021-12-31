#include <esptools/esptools.h>
#include "testconfig.h"
#include "memory_mapping.h"
#include <stdlib.h>
#include <stddef.h>
#include <string.h>

int main(void) {
    struct esp_map_file_byname_result mapping =
        esp_map_file_ro_byname(ESP_PATH_LITERAL(TEST_DATA_PATH) "/empty.esm");
    assert(mapping.addr);
    struct esp_record* rcd = mapping.addr;
    assert(strncmp(rcd->type, "TES4", 4) == 0);
    assert(rcd->data_size == 52);
    assert(rcd->flags == 1);
    assert(rcd->formID == 0);
    assert(rcd->timestamp == 0);
    assert(rcd->vcs_info == 0);
    assert(rcd->internal_version == 44);
    assert(rcd->unknown == 0);

    struct esp_field* tes4_field = (struct esp_field*)rcd->data;
    assert(strncmp(tes4_field->type, "HEDR", 4) == 0);
    assert(tes4_field->field_size == 12);

    return EXIT_SUCCESS;
}