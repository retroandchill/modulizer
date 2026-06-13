find_package(Catch2 CONFIG REQUIRED)

add_executable(modulizer_cpp_tests
        builder_test.cpp)

target_link_libraries(modulizer_cpp_tests PRIVATE Catch2::Catch2WithMain modulizer_cpp)

list(APPEND CMAKE_MODULE_PATH ${catch2_SOURCE_DIR}/extra)
include(Catch)
catch_discover_tests(modulizer_cpp_tests)
