# Copyright (c) Charles Barto
# SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

cmake_minimum_required(VERSION 3.22.1)
project(esptools LANGUAGES C)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_EXTENSIONS ON)
include(CTest)

set(GCC_OPTIONS
# warnings
-Wall -Wextra
-Wformat=2 -Wformat-overflow=2
-Wformat-signedness -Wformat-truncation=2
-Wstrict-aliasing=1
# switches
-fms-extensions
)

add_compile_options(
    "$<$<COMPILE_LANG_AND_ID:C,MSVC>:/W4>"
    "$<$<COMPILE_LANG_AND_ID:C,GNU>:${GCC_OPTIONS}>"
)

add_library(esptools SHARED)
target_include_directories(esptools PUBLIC include)
target_sources(esptools PRIVATE
    src/memory_mapping.c
    src/fileio.c
    src/parseutils.c
    src/compact.c
    src/records.c
    src/rbtree.c
    )
target_compile_definitions(esptools PRIVATE
    $<$<STREQUAL:$<TARGET_PROPERTY:esptools,TYPE>,STATIC_LIBRARY>:ESP_STATIC>
)
if(BUILD_TESTING)
    target_compile_definitions(esptools PRIVATE ESP_TEST_EXPORTS)
endif()


set(TEST_DATA_PATH ${CMAKE_CURRENT_SOURCE_DIR}/tests/testdata)
configure_file(tests/testconfig.h.in testconfig.h @ONLY)
set(CMAKE_INCLUDE_CURRENT_DIR ON)


link_libraries(esptools)
include_directories(src)
add_executable(test_just_compile tests/test_just_compile.c)
add_executable(test_memory_mapping tests/test_memory_mapping.c)
add_executable(test_empty_esp_header tests/test_empty_esp_header.c)

add_test(NAME test_memory_mapping COMMAND test_memory_mapping)
add_test(NAME test_empty_esp_header COMMAND test_empty_esp_header)