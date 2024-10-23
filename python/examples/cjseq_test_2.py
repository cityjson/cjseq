import json
import time

import cjseqpy

# f = open("/Users/hugo/data/cityjson/CityJSONSeq-demo-files/3DBAG.city.jsonl")
f = open("/Users/hugo/data/cityjson/CityJSONSeq-demo-files/3DBAG.city.jsonl")
# f = open("/Users/hugo/data/cityjson/CityJSONSeq-demo-files/NYC.jsonl")
start_time = time.time()
for i, line in enumerate(f.readlines()):
    # print("---")
    # print(line)
    if i == 0:
        cj = cjseqpy.CityJSON(line)
    else:
        a = cj.add_cjfeature_str(line)
        # print("a", a)
        # j = json.loads(line)
        # cj.add_cjfeature_json(j)
end_time = time.time()
print(end_time - start_time)
# print(cj)

start_time = time.time()
j2 = json.loads(cj.get_string())
# print(j2)
print(len(j2["CityObjects"]))
end_time = time.time()
print(end_time - start_time)

start_time = time.time()
j3 = cj.get_json()
print(len(j3["CityObjects"]))
end_time = time.time()
print(end_time - start_time)

# m = cj.cat_metadata()
# print(m)
# i = 0
# while True:
#     try:
#         f = cj.cat_feature(i)
#         print(f)
#         i += 1
#     except:
#         break
