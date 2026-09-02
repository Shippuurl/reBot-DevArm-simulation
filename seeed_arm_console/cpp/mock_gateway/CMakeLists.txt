cmake_minimum_required(VERSION 3.20)
project(arm_console_mock_gateway LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(ARM_CONSOLE_WITH_MUJOCO "Build the MuJoCo simulation driver" OFF)
option(ARM_CONSOLE_WITH_GRPC "Build the ArmGateway gRPC service" OFF)

add_executable(arm_console_mock_gateway
    src/main.cpp
    src/simulation_factory.cpp
    src/simulation_mock.cpp
)

if(ARM_CONSOLE_WITH_MUJOCO)
    set(MUJOCO_ROOT "" CACHE PATH "MuJoCo installation prefix")
    find_path(MUJOCO_INCLUDE_DIR mujoco/mujoco.h
        HINTS "${MUJOCO_ROOT}/include" /opt/mujoco/include)
    find_library(MUJOCO_LIBRARY mujoco
        HINTS "${MUJOCO_ROOT}/lib" /opt/mujoco/lib)
    if(NOT MUJOCO_INCLUDE_DIR OR NOT MUJOCO_LIBRARY)
        message(FATAL_ERROR "MuJoCo not found. Set MUJOCO_ROOT to the extracted MuJoCo directory.")
    endif()
    target_sources(arm_console_mock_gateway PRIVATE src/simulation_mujoco.cpp)
    target_compile_definitions(arm_console_mock_gateway PRIVATE ARM_CONSOLE_WITH_MUJOCO=1)
    target_include_directories(arm_console_mock_gateway PRIVATE "${MUJOCO_INCLUDE_DIR}")
    target_link_libraries(arm_console_mock_gateway PRIVATE "${MUJOCO_LIBRARY}" ${CMAKE_DL_LIBS})
endif()

if(ARM_CONSOLE_WITH_GRPC)
    # Ubuntu's protobuf development package ships the CMake FindProtobuf
    # module (while gRPC is provided as a config package).
    find_package(Protobuf REQUIRED)
    find_package(gRPC CONFIG REQUIRED)
    set(ARM_CONSOLE_PROTO "${CMAKE_CURRENT_SOURCE_DIR}/../../protocol/arm_console.proto")
    set(GRPC_GEN_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    file(MAKE_DIRECTORY "${GRPC_GEN_DIR}")
    add_custom_command(
        OUTPUT "${GRPC_GEN_DIR}/arm_console.pb.cc" "${GRPC_GEN_DIR}/arm_console.pb.h"
               "${GRPC_GEN_DIR}/arm_console.grpc.pb.cc" "${GRPC_GEN_DIR}/arm_console.grpc.pb.h"
        COMMAND protobuf::protoc --proto_path="${CMAKE_CURRENT_SOURCE_DIR}/../../protocol"
                --cpp_out="${GRPC_GEN_DIR}" --grpc_out="${GRPC_GEN_DIR}"
                --plugin=protoc-gen-grpc="$<TARGET_FILE:gRPC::grpc_cpp_plugin>" "${ARM_CONSOLE_PROTO}"
        DEPENDS "${ARM_CONSOLE_PROTO}"
    )
    target_sources(arm_console_mock_gateway PRIVATE
        src/grpc_server.cpp
        "${GRPC_GEN_DIR}/arm_console.pb.cc"
        "${GRPC_GEN_DIR}/arm_console.grpc.pb.cc")
    target_include_directories(arm_console_mock_gateway PRIVATE "${GRPC_GEN_DIR}")
    target_link_libraries(arm_console_mock_gateway PRIVATE gRPC::grpc++ protobuf::libprotobuf)
    target_compile_definitions(arm_console_mock_gateway PRIVATE ARM_CONSOLE_WITH_GRPC=1)
endif()

if(WIN32)
    target_link_libraries(arm_console_mock_gateway PRIVATE ws2_32)
endif()

if(MSVC)
    target_compile_options(arm_console_mock_gateway PRIVATE /W4 /permissive-)
else()
    target_compile_options(arm_console_mock_gateway PRIVATE -Wall -Wextra -Wpedantic)
endif()
