#pragma once
#include <esptools/macros.h>
#include <stdlib.h>
#include <assert.h>

ESP_WARNING_PUSH
ESP_BEGIN_DECLS

typedef int valuetype;

struct esp_int_grow_array {
  size_t len;
  size_t cap;
  valuetype* data;
};

void esp_int_grow_array_init(struct esp_int_grow_array* arr, valuetype cap) {
  arr->data = malloc(cap * sizeof(valuetype));
  arr->len = 0;
  arr->cap = cap;
}
struct esp_int_grow_array* esp_int_grow_array_new(valuetype cap) {
  struct esp_int_grow_array* result = malloc(sizeof(struct esp_int_grow_array));
  esp_int_grow_array_init(result, cap);
  return result;
}
void esp_int_grow_array_destroy(struct esp_int_grow_array* arr) {
  free(arr->data);
}
void esp_int_grow_array_free(struct esp_int_grow_array* arr) {
  esp_int_grow_array_destroy(arr);
  free(arr);
}

static void esp_int_grow_array_grow(struct esp_int_grow_array* arr) {
  size_t new_cap = arr->cap * 1.333;
  new_cap = new_cap < 10 ? 10 : new_cap;
  arr->data = realloc(arr->data, new_cap);
  arr->cap = new_cap;
}

void esp_int_grow_array_push(struct esp_int_grow_array* arr, valuetype val) {
  assert(arr->len <= arr->cap);
  if(arr->len == arr->cap) {
    esp_int_grow_array_grow(arr);
  }
  assert(arr->len < arr->cap);
  arr->data[arr->len] = val;
  ++arr->len;
}

ESP_END_DECLS
ESP_WARNING_POP