#include <stdbool.h>
#include <assert.h>
#include <stdlib.h>
#include <string.h>

#define ENABLE_EXPENSIVE_ASSERT 1
#if ENABLE_EXPENSIVE_ASSERT
  #define expensive_assert(expr) assert(expr)
#else
  #define expensive_assert(expr)
#endif

typedef int valtype;
typedef int keytype;


struct node {
  struct node *left;
  struct node *right;
  keytype key;
  valtype value;
};
struct node *node_new(void) {
  struct node *result = malloc(sizeof(struct node));
  memset(result, 0, sizeof(struct node));
  return result;
}
struct node *node_new_with_data(keytype key, valtype value) {
  struct node *result = node_new();
  result->key = key;
  result->value = value;
  return result;
}
static bool is_binary_tree(struct node *root) {
  bool result = 1;
  if (root && root->left) {
    result = is_binary_tree(root->left);
    result = result && root->left->key < root->key;
  }
  if (root && root->right) {
    result = result && root->right->key >= root->key;
    result = result && is_binary_tree(root->right);
  }
  return result;
}
void node_free(struct node *node) { free(node); }
void insert_node(struct node **root,
                                struct node *new) {
  expensive_assert(is_binary_tree(*root));
  struct node *cur = *root;
  struct node *par = *root;
  while (cur) {
    if (new->key < cur->key) {
      if (cur->left) {
        par = cur;
        cur = cur->left;
      } else {
        break;
      }
    } else if (new->key > cur->key) {
      if (cur->right) {
        par = cur;
        cur = cur->right;
      } else {
        break;
      }
    } else {
      new->left = cur->left;
      new->right = cur->right;
      if (par->left == cur)
        par->left = new;
      else if (par->right == cur)
        par->right = new;
      else
	assert(false);
      node_free(cur);
      return;
    }
  }
  if (cur == 0)
    *root = new;
  else if (new->key < cur->key) {
    cur->left = new;
  } else {
    cur->right = new;
  }
  expensive_assert(is_binary_tree(*root));
}
static void remove_node(struct node **node) {
  expensive_assert(is_binary_tree(*node));
  struct node *temp = *node;
  if (temp->right == 0) {
    *node = temp->left;
    goto finish;
  }
  struct node *right = (*node)->right;
  if (right->left == 0) {
    right->left = temp->left;
    *node = right;
    goto finish;
  }
  struct node *succ;
  for (succ = right->left; succ->left != 0; right = succ, succ = succ->left);
  assert(succ->left == 0);
  assert(right->left == succ);
  succ->left = temp->left;
  right->left = succ->right;
  succ->right = temp->right;
  *node = succ;
finish:
  node_free(temp);
  expensive_assert(is_binary_tree(*node));
}
