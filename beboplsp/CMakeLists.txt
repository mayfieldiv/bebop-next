add_library(beboplsp STATIC
    src/beboplsp.cpp
    src/server.cpp
    src/workspace.cpp
)

target_link_libraries(beboplsp PUBLIC lsp bebop::bebop)

target_include_directories(beboplsp
    PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include
    PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src
)

if(WIN32)
    target_link_libraries(beboplsp PRIVATE Ws2_32)
endif()

set_target_properties(beboplsp PROPERTIES
    CXX_STANDARD 20
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
    POSITION_INDEPENDENT_CODE ON
)
