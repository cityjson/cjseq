cmake_minimum_required(VERSION 3.21)
project(cjseq_cpp VERSION 0.1.0 LANGUAGES CXX)

include(CTest)

# Enable compile_commands.json for tooling.
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Allow automatically picking up the vcpkg toolchain file when VCPKG_ROOT is set.
if(NOT DEFINED CMAKE_TOOLCHAIN_FILE AND DEFINED ENV{VCPKG_ROOT})
    set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake" CACHE STRING "" FORCE)
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(nlohmann_json CONFIG REQUIRED)

add_library(cjseq STATIC
    src/cityjson.cpp
)

set_target_properties(cjseq PROPERTIES EXPORT_NAME cjseq)

target_include_directories(cjseq
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(cjseq
    PUBLIC
        nlohmann_json::nlohmann_json
)

target_compile_features(cjseq PUBLIC cxx_std_20)

add_executable(cjseq_cli src/main.cpp)

target_link_libraries(cjseq_cli
    PRIVATE
        cjseq
)

target_compile_features(cjseq_cli PRIVATE cxx_std_20)

if(BUILD_TESTING)
    add_executable(cjseq_tests
        tests/cityjson_tests.cpp
    )

    target_link_libraries(cjseq_tests
        PRIVATE
            cjseq
    )

    target_compile_features(cjseq_tests PRIVATE cxx_std_20)

    add_test(NAME cjseq_tests COMMAND cjseq_tests)
endif()
