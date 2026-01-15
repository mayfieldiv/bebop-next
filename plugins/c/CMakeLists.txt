add_executable(bebopc-gen-c src/generator.c)

target_link_libraries(bebopc-gen-c PRIVATE bebop)

set_target_properties(bebopc-gen-c PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bebopc"
)

install(TARGETS bebopc-gen-c RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
