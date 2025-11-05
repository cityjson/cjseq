# cjseq C++ Prototype

This directory hosts a native C++ build that mirrors the Rust functionality incrementally. It uses CMake and vcpkg for dependency management and pulls in [nlohmann/json](https://github.com/nlohmann/json) for JSON handling.

## Prerequisites

- CMake 3.21+
- A C++20-compatible compiler (clang++ 15+, MSVC 19.3+, or g++ 11+)
- [vcpkg](https://github.com/microsoft/vcpkg)

If `VCPKG_ROOT` is defined in your environment, the project automatically picks up the vcpkg toolchain file.

Install the dependency once:

```bash
${VCPKG_ROOT}/vcpkg install nlohmann-json
```

## Configure and Build

```bash
cmake -S cpp -B cpp/build -DCMAKE_TOOLCHAIN_FILE=${VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake
cmake --build cpp/build
```

This builds the static library `cjseq` along with a simple CLI `cjseq_cli` that reports the number of CityObjects in a CityJSON file.

## Run the Example

```bash
./cpp/build/cjseq_cli data/3dbag_b2.city.json
```

Use this as the entry point for expanding the C++ API to feature parity with the Rust crate.

