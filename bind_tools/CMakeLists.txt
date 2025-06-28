link_libraries(Qt6::Core)

add_executable(bind_list bind_list.cpp)
target_link_libraries(bind_list PRIVATE bindlib)
add_executable(bind_remove bind_remove.cpp)
