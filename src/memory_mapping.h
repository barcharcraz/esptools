#include <esptools/macros.h>
#include <esptools/types.h>

ESP_WARNING_PUSH
ESP_BEGIN_DECLS

ESP_EXPORT_TEST void* esp_map_file_ro(size_t len, espnativefd fd);

ESP_EXPORT_TEST int esp_unmap_file(void* addr, size_t len);

struct esp_map_file_byname_result {
    void* addr;
    size_t len;
};

ESP_EXPORT_TEST
struct esp_map_file_byname_result 
esp_map_file_ro_byname(esppathchar* filepath);

ESP_EXPORT_TEST
int esp_unmap_file_byname(struct esp_map_file_byname_result to_unmap);


ESP_END_DECLS
ESP_WARNING_POP