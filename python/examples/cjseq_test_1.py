import json
import time

import cjseqpy

# f = open("/Users/hugo/data/cityjson/CityJSONSeq-demo-files/3DBAG.city.jsonl")
# f = open("/Users/hugo/data/cityjson/CityJSONSeq-demo-files/3DBAG.city.jsonl")
# f = open("/Users/hugo/data/cityjson/CityJSONSeq-demo-files/NYC.json")
f = open("/Users/hugo/data/cityjson/CityJSONSeq-demo-files/3DBAG.city.json")
start_time = time.time()
cj = cjseqpy.CityJSON(f.read())
end_time = time.time()
print(end_time - start_time)
# print(cj)

# start_time = time.time()
# j2 = json.loads(cj.get_cj_str())
# # print(j2)
# # print(len(j2["CityObjects"]))
# end_time = time.time()
# print(end_time - start_time)

# start_time = time.time()
# j2 = orjson.loads(cj.get_cj_str())
# # print(j2)
# # print(len(j2["CityObjects"]))
# end_time = time.time()
# print(end_time - start_time)


# start_time = time.time()
# j3 = cj.get_cj_json()
# # print(len(j3["CityObjects"]))
# end_time = time.time()
# print(end_time - start_time)

start_time = time.time()
# m = cj.get_metadata_str()
i = 0
while True:
    try:
        f = cj.get_cjfeature_json(i)
        # print(f)
        i += 1
    except:
        print(i)
        break
end_time = time.time()
print(end_time - start_time)

start_time = time.time()
# m = cj.get_metadata_str()
i = 0
while True:
    try:
        f = json.loads(cj.get_cjfeature_str(i))
        # print(f)
        i += 1
    except:
        print(i)
        break
end_time = time.time()
print(end_time - start_time)
