cmake_minimum_required(VERSION 3.15)
project(OpenVINO_NPU_Standalone VERSION 1.0.0)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Library
add_library(openvino_npu_standalone SHARED
    src/npu_plugin.cpp
)

# Include directories
target_include_directories(openvino_npu_standalone PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Link against OpenVINO libraries
if(WIN32)
    target_link_libraries(openvino_npu_standalone
        ${CMAKE_CURRENT_SOURCE_DIR}/lib/libopenvino.dll.a
        ${CMAKE_CURRENT_SOURCE_DIR}/lib/libopenvino_intel_npu_plugin.dll.a
        ${CMAKE_CURRENT_SOURCE_DIR}/lib/libtbb12.dll.a
    )
else()
    target_link_libraries(openvino_npu_standalone
        ${CMAKE_CURRENT_SOURCE_DIR}/lib/libopenvino.so
        ${CMAKE_CURRENT_SOURCE_DIR}/lib/libopenvino_intel_npu_plugin.so
        ${CMAKE_CURRENT_SOURCE_DIR}/lib/libtbb.so.12
    )
endif()

# Install
install(TARGETS openvino_npu_standalone
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
)

install(FILES include/npu_plugin.h
    DESTINATION include
)

# Test executable
add_executable(test_npu_plugin tests/test_basic.cpp)
target_link_libraries(test_npu_plugin openvino_npu_standalone)
