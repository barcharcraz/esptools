# Copyright (c) Charles Barto
# SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only

cmake_minimum_required(VERSION 3.22.1)
project(esptools LANGUAGES C)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_EXTENSIONS ON)


add_library(esptools INTERFACE)
target_include_directories(esptools INTERFACE include)

link_libraries(esptools)

add_executable(test_just_compile tests/just_compile.c)