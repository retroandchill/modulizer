# @file CMakeLists.txt
#
# @copyright Copyright (c) 2026 Retro & Chill. All rights reserved.
# Licensed under the MIT License. See LICENSE file in the project root for full license information.
cmake_minimum_required(VERSION 4.1)

set(CMAKE_EXPERIMENTAL_CXX_IMPORT_STD "d0edc3af-4c50-42ea-a356-e2862fe7a444")


option(MODULIZER_BUILD_SHARED "Build as a shared library" OFF)

option(MODULIZER_BUILD_TESTS "Build tests for the library" ON)

if (MODULIZER_BUILD_SHARED)
    set(BUILD_SHARED_LIBS ON)
else ()
    set(BUILD_SHARED_LIBS OFF)

    set(VCPKG_CRT_LINKAGE static)
    set(VCPKG_LIBRARY_LINKAGE static)
endif ()

set(VCPKG_DEPS_DIR ${CMAKE_BINARY_DIR}/vcpkg_installed/${VCPKG_TARGET_TRIPLET})
set(VCPKG_BIN_DIR ${VCPKG_DEPS_DIR}/bin)

if (MODULIZER_BUILD_SHARED)
    add_compile_definitions(MODULIZER_SHARED MODULIZER_EXPORT)
endif()

project(modulizer LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_MODULE_STD ON)
if (MSVC)
    add_compile_definitions(_UTF8)
endif()

include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include)

include(FetchContent)

FetchContent_Declare(
        Corrosion
        GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
        GIT_TAG v0.6.1 # Optionally specify a commit hash, version tag or branch here
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml
        CRATE_TYPES staticlib)

if (MODULIZER_BUILD_TESTS)
    include(CTest)
    enable_testing()
endif()

add_subdirectory(bindings/cpp)
