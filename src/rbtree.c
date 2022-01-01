#include "rbtree.h"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

#define ESP_RBTREE_INST(pfx, instname, type)                                   \
  struct pfx##_##instname##_rbtree_node {                                      \
    struct pfx##_##instname##_rbtree_node *left;                               \
    struct pfx##_##instname##_rbtree_node *right;                              \
    type data;                                                                 \
  };                                                                           \
  struct pfx##_##instname##_rbtree_node *pfx##_##insname##_rbtree_node_new(    \
      void) {                                                                  \
    return malloc(sizeof(struct pfx##_##instname##_rbtree_node));              \
  }

enum esp_rbtree_color { ESP_RBTREE_COLOR_RED, ESP_RBTREE_COLOR_BLACK };

struct esp_int_rbtree_node {
  struct esp_int_rbtree_node *left;
  struct esp_int_rbtree_node *right;
  int key;
  int value;
  uint8_t color;
};

struct esp_int_rbtree_node *esp_int_rbtree_node_new(void) {
  struct esp_int_rbtree_node *result =
      malloc(sizeof(struct esp_int_rbtree_node));
  memset(result, 0, sizeof(struct esp_int_rbtree_node));
  return result;
}

struct esp_int_rbtree_node *esp_int_rbtree_node_new_with_data(int key,
                                                              int value) {
  struct esp_int_rbtree_node *result = esp_int_rbtree_node_new();
  result->key = key;
  result->value = value;
  return result;
}

void esp_int_rbtree_node_free(struct esp_int_rbtree_node *node) { free(node); }

int esp_int_rbtree_insert(struct esp_int_rbtree_node *root, int key,
                          int value) {
  struct esp_int_rbtree_node *cur = root;
  struct esp_int_rbtree_node *child = root;
  while (child) {
    cur = child;
    if (key < cur->key) {
      child = cur->left;
    } else if (key > cur->key) {
      child = cur->right;
    } else {
      int old = cur->value;
      cur->value = value;
      return old;
    }
  }
  struct esp_int_rbtree_node *new_node =
      esp_int_rbtree_node_new_with_data(key, value);
  if (key < cur->key) {
    cur->left = new_node;
  } else {
    cur->right = new_node;
  }
}
bool esp_int_rbtree_is_binary_tree(struct esp_int_rbtree_node* root) {
  bool result = false;
  if(root->left) {
    result = esp_int_rbtree_is_binary_tree(root->left);
    result = result && root->left->key < root->key;
  }
  if(root->right) {
    result = result && root->right->key >= root->key;
    result = result && esp_int_rbtree_is_binary_tree(root->right);
  }
  return result;
}
int esp_int_rbtree_remove(struct esp_int_rbtree_node *root, int key) {
  return 0;
}