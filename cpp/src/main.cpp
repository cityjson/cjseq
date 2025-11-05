#include "cjseq/cityjson.hpp"

#include <fstream>
#include <iostream>
#include <stdexcept>

namespace {
std::string read_file(const std::string& path) {
    std::ifstream input(path);
    if (!input.is_open()) {
        throw std::runtime_error("Failed to open file: " + path);
    }
    std::string content;
    input.seekg(0, std::ios::end);
    content.reserve(static_cast<size_t>(input.tellg()));
    input.seekg(0, std::ios::beg);
    content.assign((std::istreambuf_iterator<char>(input)), std::istreambuf_iterator<char>());
    return content;
}
} // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <cityjson-file>\n";
        return 1;
    }

    try {
        const std::string json_text = read_file(argv[1]);
        const auto parsed = cjseq::parse_cityjson(json_text);
        std::cout << "Parsed CityJSON document contains "
                  << parsed["CityObjects"].size() << " objects" << std::endl;
    } catch (const std::exception& ex) {
        std::cerr << "Error: " << ex.what() << std::endl;
        return 1;
    }

    return 0;
}

