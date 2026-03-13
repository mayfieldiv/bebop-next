find_program(CARGO_EXECUTABLE cargo)
if(NOT CARGO_EXECUTABLE)
    message(STATUS "Cargo not found, skipping bebopc-gen-rust")
    return()
endif()

if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set(CARGO_BUILD_TYPE "--release")
    set(CARGO_TARGET_DIR "release")
else()
    set(CARGO_BUILD_TYPE "")
    set(CARGO_TARGET_DIR "debug")
endif()

set(RUST_PLUGIN_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(RUST_TARGET_DIR "${RUST_PLUGIN_DIR}/target/${CARGO_TARGET_DIR}")
set(RUST_GEN_RUST "${RUST_TARGET_DIR}/bebopc-gen-rust${CMAKE_EXECUTABLE_SUFFIX}")

add_custom_target(bebopc-gen-rust ALL
    COMMAND ${CARGO_EXECUTABLE} build
        ${CARGO_BUILD_TYPE}
        --manifest-path "${RUST_PLUGIN_DIR}/Cargo.toml"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${RUST_GEN_RUST}"
        "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/bebopc-gen-rust${CMAKE_EXECUTABLE_SUFFIX}"
    COMMENT "Building bebopc-gen-rust"
    WORKING_DIRECTORY "${RUST_PLUGIN_DIR}"
    VERBATIM
)

install(PROGRAMS "${RUST_GEN_RUST}" DESTINATION ${CMAKE_INSTALL_BINDIR})
