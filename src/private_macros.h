// Copyright (c) Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

#pragma once

#ifdef ESP_ENABLE_EXPENSIVE_CHECKS
#define esp_expensive_assert(expression) assert(expression)
#else
#define esp_expensive_assert(expression)
#endif

#define ESP_INSTANTIATE_COMPONENT_ASSERTS(component_name)                      \
  X(esp_##compname##_assert)                                                   \
  X(esp_##compname##_expensive_assert)

