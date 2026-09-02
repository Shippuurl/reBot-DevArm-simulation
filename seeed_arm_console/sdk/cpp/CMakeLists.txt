cmake_minimum_required(VERSION 3.20)
project(rebot_arm_sdk LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

set(REBOT_PROTO "${CMAKE_CURRENT_SOURCE_DIR}/../../protocol/arm_console.proto")
set(REBOT_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${REBOT_GENERATED_DIR}")

add_custom_command(
    OUTPUT "${REBOT_GENERATED_DIR}/arm_console.pb.cc"
           "${REBOT_GENERATED_DIR}/arm_console.pb.h"
           "${REBOT_GENERATED_DIR}/arm_console.grpc.pb.cc"
           "${REBOT_GENERATED_DIR}/arm_console.grpc.pb.h"
    COMMAND protobuf::protoc
            --proto_path=${CMAKE_CURRENT_SOURCE_DIR}/../../protocol
            --cpp_out=${REBOT_GENERATED_DIR}
            --grpc_out=${REBOT_GENERATED_DIR}
            --plugin=protoc-gen-grpc=$<TARGET_FILE:gRPC::grpc_cpp_plugin>
            "${REBOT_PROTO}"
    DEPENDS "${REBOT_PROTO}"
    VERBATIM
)

add_library(rebot_arm_sdk STATIC
    src/client.cpp
    "${REBOT_GENERATED_DIR}/arm_console.pb.cc"
    "${REBOT_GENERATED_DIR}/arm_console.grpc.pb.cc"
)
target_include_directories(rebot_arm_sdk
    PUBLIC "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>"
           "$<INSTALL_INTERFACE:include>"
    PRIVATE "${REBOT_GENERATED_DIR}"
)
target_link_libraries(rebot_arm_sdk PUBLIC gRPC::grpc++ protobuf::libprotobuf)
target_compile_options(rebot_arm_sdk PRIVATE -Wall -Wextra -Wpedantic)

option(REBOT_SDK_BUILD_EXAMPLE "Build the SDK smoke example" ON)
if(REBOT_SDK_BUILD_EXAMPLE)
    add_executable(rebot_sdk_gateway_example examples/gateway_client.cpp)
    target_link_libraries(rebot_sdk_gateway_example PRIVATE rebot_arm_sdk)
    add_executable(rebot_sdk_planner_example examples/planner_client.cpp)
    target_link_libraries(rebot_sdk_planner_example PRIVATE rebot_arm_sdk)
endif()

install(TARGETS rebot_arm_sdk EXPORT rebot_arm_sdk_targets)
install(DIRECTORY include/ DESTINATION include)
install(EXPORT rebot_arm_sdk_targets
        FILE rebot_arm_sdkTargets.cmake
        NAMESPACE rebot::
        DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/rebot_arm_sdk")
configure_package_config_file(
    cmake/rebot_arm_sdkConfig.cmake.in
    "${CMAKE_CURRENT_BINARY_DIR}/rebot_arm_sdkConfig.cmake"
    INSTALL_DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/rebot_arm_sdk")
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/rebot_arm_sdkConfigVersion.cmake"
    VERSION 0.1.0
    COMPATIBILITY SameMajorVersion)
install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/rebot_arm_sdkConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/rebot_arm_sdkConfigVersion.cmake"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/rebot_arm_sdk")
