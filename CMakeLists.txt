cmake_minimum_required(VERSION 4.0.3)
set(CMAKE_EXPERIMENTAL_CXX_IMPORT_STD "d0edc3af-4c50-42ea-a356-e2862fe7a444")

project(mobl VERSION 0.0.1 LANGUAGES C CXX)
set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_EXTENSIONS ON)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_MODULE_STD ON)
set(CMAKE_AUTOMOC ON)


enable_testing()

find_package(Qt6 REQUIRED COMPONENTS Core Test DBus)

find_package(Threads)
find_library(BINDFLTAPI_LIB bindfltapi PATHS ${CMAKE_CURRENT_LIST_DIR} NO_DEFAULT_PATH)

target_link_libraries(__cmake_cxx26 PRIVATE Threads::Threads)

qt_add_library(bindlib OBJECT)
target_sources(bindlib PUBLIC FILE_SET CXX_MODULES FILES bindlib.cppm)
target_link_libraries(bindlib PUBLIC ${BINDFLTAPI_LIB} Threads::Threads)



qt_add_library(repo OBJECT)
target_sources(repo PUBLIC FILE_SET CXX_MODULES FILES repo.cpp)
target_link_libraries(repo PRIVATE Qt6::Core Qt6::DBus Threads::Threads)

add_executable(alltests repo_tests.cpp)
target_link_libraries(alltests PRIVATE Qt6::Core Qt6::Test Threads::Threads repo)
add_test(NAME alltests COMMAND alltests)
add_subdirectory(bind_tools)
