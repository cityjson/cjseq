#include "cjseq/cityjson.hpp"

namespace cjseq {

JsonValue parse_cityjson(const std::string &json_text) {
  return nlohmann::json::parse(json_text);
}

} // namespace cjseq
