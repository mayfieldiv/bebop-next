add_executable(bebopc-gen-c src/generator.c)

target_link_libraries(bebopc-gen-c PRIVATE bebop)

install(TARGETS bebopc-gen-c RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
