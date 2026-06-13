set(MODULIZER_MODULES
        src/modulizer.ixx
        src/core.ixx
        src/native.ixx
        src/options.ixx)

add_library(modulizer_cpp ${MODULIZER_MODULES})

target_link_libraries(modulizer_cpp PUBLIC modulizer)

target_sources(modulizer_cpp PUBLIC
        FILE_SET modulizer_public
        TYPE CXX_MODULES
        FILES
        ${MODULIZER_MODULES})

if (MODULIZER_BUILD_TESTS)
    add_subdirectory(tests)
endif()
