cmake_minimum_required(VERSION 3.21)
project(rtcore_barrier LANGUAGES CXX CUDA)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CUDA_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

find_package(CUDAToolkit REQUIRED)

if(DEFINED OPTIX_HOME)
  set(_OPTIX_HINTS ${OPTIX_HOME})
elseif(DEFINED ENV{OPTIX_HOME})
  set(_OPTIX_HINTS $ENV{OPTIX_HOME})
endif()

find_path(OPTIX_INCLUDE_DIR optix.h HINTS ${_OPTIX_HINTS} PATH_SUFFIXES include)
if(NOT OPTIX_INCLUDE_DIR)
  message(FATAL_ERROR "Could not find optix.h. Set OPTIX_HOME to your OptiX SDK root.")
endif()

set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/ptx_path.h.in ${CMAKE_CURRENT_BINARY_DIR}/ptx_path.h @ONLY)

add_custom_command(
  OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/rtcore_barrier_device.ptx
  COMMAND ${CMAKE_CUDA_COMPILER}
          --ptx
          -std=c++17
          -lineinfo
          -I${OPTIX_INCLUDE_DIR}
          -I${CUDAToolkit_INCLUDE_DIRS}
          ${CMAKE_CURRENT_SOURCE_DIR}/rtcore_barrier_device.cu
          -o ${CMAKE_CURRENT_BINARY_DIR}/rtcore_barrier_device.ptx
  DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/rtcore_barrier_device.cu
  VERBATIM)

add_custom_target(rtcore_barrier_ptx DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/rtcore_barrier_device.ptx)

add_library(rtcore_barrier SHARED rtcore_barrier_host.cpp)
add_dependencies(rtcore_barrier rtcore_barrier_ptx)

target_include_directories(rtcore_barrier PRIVATE
  ${OPTIX_INCLUDE_DIR}
  ${CUDAToolkit_INCLUDE_DIRS}
  ${CMAKE_CURRENT_SOURCE_DIR}
  ${CMAKE_CURRENT_BINARY_DIR})

target_link_libraries(rtcore_barrier PRIVATE CUDA::cudart CUDA::cuda_driver)
