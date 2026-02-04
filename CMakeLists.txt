cmake_minimum_required(VERSION 3.25)
project(bebop-vnext
    VERSION 0.1.0
    DESCRIPTION "Bebop schema compiler"
    LANGUAGES C CXX
)

include(GNUInstallDirs)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Version can be set externally via -DBEBOP_VERSION_*, otherwise detect from git/project
if(NOT DEFINED BEBOP_VERSION_MAJOR)
    find_package(Git QUIET)
    if(GIT_FOUND)
        execute_process(
            COMMAND ${GIT_EXECUTABLE} describe --tags --match "v*" --abbrev=0
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            OUTPUT_VARIABLE GIT_TAG
            OUTPUT_STRIP_TRAILING_WHITESPACE
            ERROR_QUIET
            RESULT_VARIABLE GIT_TAG_RESULT
        )
        if(GIT_TAG_RESULT EQUAL 0 AND GIT_TAG MATCHES "^v([0-9]+)\\.([0-9]+)\\.([0-9]+)(-(.+))?$")
            set(BEBOP_VERSION_MAJOR ${CMAKE_MATCH_1})
            set(BEBOP_VERSION_MINOR ${CMAKE_MATCH_2})
            set(BEBOP_VERSION_PATCH ${CMAKE_MATCH_3})
            set(BEBOP_VERSION_SUFFIX "${CMAKE_MATCH_5}")
        endif()
    endif()

    if(NOT DEFINED BEBOP_VERSION_MAJOR)
        set(BEBOP_VERSION_MAJOR ${PROJECT_VERSION_MAJOR})
        set(BEBOP_VERSION_MINOR ${PROJECT_VERSION_MINOR})
        set(BEBOP_VERSION_PATCH ${PROJECT_VERSION_PATCH})
        set(BEBOP_VERSION_SUFFIX "")
    endif()
endif()

if(BEBOP_VERSION_SUFFIX)
    set(BEBOP_VERSION_STRING "${BEBOP_VERSION_MAJOR}.${BEBOP_VERSION_MINOR}.${BEBOP_VERSION_PATCH}-${BEBOP_VERSION_SUFFIX}")
else()
    set(BEBOP_VERSION_STRING "${BEBOP_VERSION_MAJOR}.${BEBOP_VERSION_MINOR}.${BEBOP_VERSION_PATCH}")
endif()

message(STATUS "Bebop version: ${BEBOP_VERSION_STRING}")

add_compile_definitions(
    BEBOP_VERSION_MAJOR=${BEBOP_VERSION_MAJOR}
    BEBOP_VERSION_MINOR=${BEBOP_VERSION_MINOR}
    BEBOP_VERSION_PATCH=${BEBOP_VERSION_PATCH}
    BEBOP_VERSION_SUFFIX="${BEBOP_VERSION_SUFFIX}"
    BEBOP_VERSION_STRING="${BEBOP_VERSION_STRING}"
)

option(BEBOP_BUILD_TESTS "Build tests" ON)
option(BEBOP_BUILD_FRONTEND "Build bebopc frontend" ON)

if(CMAKE_C_COMPILER_ID MATCHES "Clang|GNU")
    add_compile_options(
        -Wall -Wextra -Wpedantic -Werror
        -Wconversion -Wsign-conversion -Wdouble-promotion
        -Wformat=2 -Wundef -Wshadow
        -Wcast-qual -Wcast-align
        -Wstrict-prototypes -Wmissing-prototypes
        -Wredundant-decls -Wnull-dereference
    )
    if(CMAKE_C_COMPILER_ID STREQUAL "GNU")
        add_compile_options(-Wno-format-zero-length)
    endif()
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        add_compile_options(-O2)
    else()
        add_compile_options(-g3 -O0 -fno-omit-frame-pointer)
        add_compile_options(-fsanitize=address,undefined)
        add_link_options(-fsanitize=address,undefined)
    endif()
elseif(MSVC)
    add_compile_options(/W4 /WX /utf-8)
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        add_compile_options(/O2)
    else()
        add_compile_options(/Od /Zi /fsanitize=address)
    endif()
endif()

add_subdirectory(vendor)
add_subdirectory(bebop)
add_subdirectory(beboplsp)

if(BEBOP_BUILD_FRONTEND)
    add_subdirectory(bebopc)
endif()
add_subdirectory(plugins)

if(BEBOP_BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

if(NOT TARGET uninstall)
    configure_file(
        "${CMAKE_CURRENT_SOURCE_DIR}/cmake/cmake_uninstall.cmake.in"
        "${CMAKE_CURRENT_BINARY_DIR}/cmake_uninstall.cmake"
        IMMEDIATE @ONLY
    )
    add_custom_target(uninstall
        COMMAND ${CMAKE_COMMAND} -P ${CMAKE_CURRENT_BINARY_DIR}/cmake_uninstall.cmake
    )
endif()
