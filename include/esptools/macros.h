// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once

// conditionally add stuff to make headers work in c++ compilers
#ifdef __cplusplus
#define ESP_BEGIN_DECLS extern "C" {
#define ESP_END_DECLS }
#else
#define ESP_BEGIN_DECLS
#define ESP_END_DECLS
#endif

#ifdef _MSC_VER
/* 
 * C4200: nonstandard extension used : zero-sized array in struct/union
 *     this fires upon use of FAMs even in C17 mode
 */
#define ESP_WARNING_PUSH \
    _Pragma("warning(push)") \
    _Pragma("warning(disable: 4200)")

#define ESP_WARNING_POP _Pragma("warning(pop)")
#endif

#ifdef _WIN32
#define ESP_PATH_LITERAL(l) L## "" l
#else
#define ESP_PATH_LITERAL(l) l
#endif


#ifdef ESP_STATIC
#   define ESP_EXPORT
#else
#   ifdef esptools_EXPORTS
#       ifdef _MSC_VER
#           define ESP_EXPORT __declspec(dllexport)
#       endif
#   else
#       ifdef _MSC_VER
#           define ESP_EXPORT __declspec(dllimport)
#       endif
#   endif
#endif

#ifdef ESP_TEST_EXPORTS
#define ESP_EXPORT_TEST ESP_EXPORT
#else
#define ESP_EXPORT_TEST
#endif