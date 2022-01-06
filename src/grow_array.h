#pragma once
#include <esptools/macros.h>
#include <stdlib.h>
ESP_WARNING_PUSH
ESP_BEGIN_DECLS

typedef int valuetype;

struct esp_int_grow_array {
  size_t len;
  valuetype* arr;
};

struct esp_int_grow_array_private {
  struct esp_int_grow_array;
  size_t cap;
};

void esp_int_grow_array_init(struct esp_int_grow_array* arr, valuetype cap) {
  struct esp_int_grow_array_private* priv = (struct esp_int_grow_array_private*)arr;
  priv->arr = malloc(cap * sizeof(valuetype));
  priv->len = 0;
  priv->cap = cap;
}
struct esp_int_grow_array* esp_int_grow_array_new(valuetype cap) {
  
}

void esp_int_grow_array_destroy(struct esp_int_grow_array* arr) {
  free(arr->arr);
}
void esp_int_grow_array_free(struct esp_int_grow_array* arr) {
  esp_int_grow_array_destroy(arr);
  free(arr);
}

ESP_END_DECLS
ESP_WARNING_POP