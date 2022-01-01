#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include "fileio.h"
#include <stdbool.h>
#include <Windows.h>


ESP_EXPORT_TEST bool esp_copy_file(esppathchar* src, esppathchar* dst) {
    BOOL res = CopyFileW(src, dst, TRUE);
    if(res)
        return true;
    return false;
}