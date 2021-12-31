#pragma once

// conditionally add stuff to make headers work in c++ compilers
#ifdef __cplusplus
#define ESP_BEGIN_DECLS extern "C" {
#define ESP_END_DECLS }
#else
#define ESP_BEGIN_DECLS
#define ESP_END_DECLS
#endif