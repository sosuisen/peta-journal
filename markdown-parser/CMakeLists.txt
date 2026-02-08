cmake_minimum_required(VERSION 3.16)
project(peta_markdown LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_C_STANDARD 99)

# Fix for MSVC UTF-8 encoding issues (especially on Japanese Windows)
if(MSVC)
  add_compile_options(/utf-8)
  add_compile_options(/wd4819)
endif()

# Build cmark-gfm as subdirectory
set(CMARK_TESTS OFF CACHE BOOL "" FORCE)
set(CMARK_SHARED OFF CACHE BOOL "" FORCE)
set(CMARK_STATIC ON CACHE BOOL "" FORCE)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
add_subdirectory(vendor/cmark-gfm)

# Shared library
add_library(peta_markdown SHARED src/peta_markdown.cpp)

target_include_directories(peta_markdown
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/vendor/cmark-gfm/src
        ${CMAKE_CURRENT_SOURCE_DIR}/vendor/cmark-gfm/extensions
        ${CMAKE_CURRENT_BINARY_DIR}/vendor/cmark-gfm/src
        ${CMAKE_CURRENT_BINARY_DIR}/vendor/cmark-gfm/extensions
)

# Platform-specific linking
if(MSVC)
    # MSVC: Link libraries first, then add /WHOLEARCHIVE flags
    target_link_libraries(peta_markdown
        PRIVATE
            libcmark-gfm_static
            libcmark-gfm-extensions_static
    )
    target_link_options(peta_markdown
        PRIVATE
            /WHOLEARCHIVE:$<TARGET_FILE:libcmark-gfm_static>
            /WHOLEARCHIVE:$<TARGET_FILE:libcmark-gfm-extensions_static>
    )
elseif(MINGW OR UNIX)
    # MinGW/GCC/Clang: Use -Wl,--whole-archive
    target_link_libraries(peta_markdown
        PRIVATE
            -Wl,--whole-archive
            libcmark-gfm_static
            libcmark-gfm-extensions_static
            -Wl,--no-whole-archive
    )
else()
    # Fallback: standard linking (may not work correctly)
    target_link_libraries(peta_markdown
        PRIVATE
            libcmark-gfm_static
            libcmark-gfm-extensions_static
    )
endif()

set_target_properties(peta_markdown PROPERTIES
    VERSION 1.0.0
    SOVERSION 1
    C_VISIBILITY_PRESET hidden
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN ON
)

# Enable testing
enable_testing()

# Test executable
add_executable(test_markdown test/test_markdown.cpp)

target_include_directories(test_markdown
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(test_markdown
    PRIVATE
        peta_markdown
)

# Register test with CTest
add_test(NAME markdown_parser_test COMMAND test_markdown)
