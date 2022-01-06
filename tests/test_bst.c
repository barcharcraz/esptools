#include "bst.h"
#include <string.h>
#include <assert.h>
#include <stdbool.h>
#include <stdlib.h>

ESP_BST_EMIT_DEFNS(static, int_bst, int, int)


void main(void) {
  struct int_bst_node* tree = 0;
  for(int i = 0; i < 10000; ++i) {
    int next = rand();
    struct int_bst_node* n = int_bst_node_new_with_data(next, 42);
    int_bst_insert_node(&tree, n);
  }
}