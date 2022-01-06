#pragma once
#include <esptools/macros.h>

ESP_WARNING_PUSH
ESP_BEGIN_DECLS

/* macroize with:
 * s/esp_int_bst(\w*)/instance_name##$1
 * s/$/\/
 * s/int key/keytype key/
 * s/int value/keytype value/
 */

#define ESP_BST_ENABLE_EXPENSIVE_ASSERT 1
#if ESP_BST_ENABLE_EXPENSIVE_ASSERT
  #define esp_bst_expensive_assert(expr) assert(expr)
#else
  #define esp_bst_expensive_assert(expr) ((void)0)
#endif

#define ESP_BST_EMIT_DEFNS(function_pfx, instance_name, keytype, valtype) \
\
  struct instance_name##_node { \
    struct instance_name##_node *left; \
    struct instance_name##_node *right; \
    keytype key; \
    valtype value; \
  }; \
\
  function_pfx struct instance_name##_node *instance_name##_node_new(void) { \
    struct instance_name##_node *result = \
        malloc(sizeof(struct instance_name##_node)); \
    memset(result, 0, sizeof(struct instance_name##_node)); \
    return result; \
  } \
\
  function_pfx struct instance_name##_node \
      *instance_name##_node_new_with_data(keytype key, valtype value) { \
    struct instance_name##_node *result = instance_name##_node_new(); \
    result->key = key; \
    result->value = value; \
    return result; \
  } \
\
  static bool instance_name##_is_binary_tree( \
      struct instance_name##_node *root) { \
    bool result = true; \
    if (root && root->left) { \
      result = instance_name##_is_binary_tree(root->left); \
      result = result && root->left->key < root->key; \
    } \
    if (root && root->right) { \
      result = result && root->right->key >= root->key; \
      result = result && instance_name##_is_binary_tree(root->right); \
    } \
    return result; \
  } \
\
  function_pfx void instance_name##_node_free( \
      struct instance_name##_node *node) { \
    free(node); \
  } \
\
  function_pfx void instance_name##_insert_node( \
      struct instance_name##_node **root, struct instance_name##_node *new) { \
    esp_bst_expensive_assert(instance_name##_is_binary_tree(*root)); \
    struct instance_name##_node *cur = *root; \
    struct instance_name##_node *par = *root; \
    while (cur) { \
      if (new->key < cur->key) { \
        if (cur->left) { \
          par = cur; \
          cur = cur->left; \
        } else { \
          break; \
        } \
      } else if (new->key > cur->key) { \
        if (cur->right) { \
          par = cur; \
          cur = cur->right; \
        } else { \
          break; \
        } \
      } else { \
        new->left = cur->left; \
        new->right = cur->right; \
        if (par->left == cur) \
          par->left = new; \
        else if (par->right == cur) \
          par->right = new; \
        else \
          assert(false); \
        instance_name##_node_free(cur); \
        return;\
      } \
    } \
    if (cur == 0) \
      *root = new; \
    else if (new->key < cur->key) { \
      cur->left = new; \
    } else { \
      cur->right = new; \
    } \
    esp_bst_expensive_assert(instance_name##_is_binary_tree(*root)); \
  } \
\
  function_pfx void instance_name##_remove_node( \
      struct instance_name##_node **node) { \
    esp_bst_expensive_assert(instance_name##_is_binary_tree(*node)); \
    struct instance_name##_node *temp = *node; \
    if (temp->right == 0) { \
      /* node is the biggest in subtree, new subtree is */ \
      /* exactly the left node */ \
      *node = temp->left; \
      goto finish; \
    } \
    struct instance_name##_node *right = (*node)->right; \
    if (right->left == 0) { \
      right->left = temp->left; \
      *node = right; \
      goto finish; \
    } \
    struct instance_name##_node *succ; \
    for (succ = right->left; succ->left != 0; right = succ, succ = succ->left) \
      ; \
    assert(succ->left == 0); \
    assert(right->left == succ); \
    succ->left = temp->left; \
    right->left = succ->right; \
    succ->right = temp->right; \
    *node = succ; \
  finish: \
    instance_name##_node_free(temp); \
    esp_bst_expensive_assert(instance_name##_is_binary_tree(*node)); \
  }

#define ESP_BST_EMIT_DECLS(function_pfx, instance_name, keytype, valtype) \
  struct instance_name##_node; \
  struct instance_name##_node *instance_name_node_new(void); \
  struct instance_name##_node *instance_name_node_new_with_data( \
      keytype key, keytype value); \
  void instance_name_node_free(struct instance_name##_node *node); \
  int instance_name_insert_node(struct instance_name##_node **root, \
                                struct instance_name##_node *new); \
  void instance_name_remove_node(struct instance_name##_node **node);

ESP_END_DECLS
ESP_WARNING_POP
