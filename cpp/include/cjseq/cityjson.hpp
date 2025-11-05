#pragma once

#include <string>

#include <nlohmann/json.hpp>

namespace cjseq {

using JsonValue = nlohmann::json;

JsonValue parse_cityjson(const std::string &json_text);

} // namespace cjseq

