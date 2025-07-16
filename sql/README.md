# sidepath estimation sql scripts

The psql script [generate_sidepath_estimation.script.sql](./generate_sidepath_estimation.script.sql) calculates an *is_sidepath* estimate analogously to the original estimation [CQI](https://github.com/SupaplexOSM/OSM-Cycling-Quality-Index). The estimation is based on evenly distributed checkpoint-buffers along paths  (e.g., with distance 100m and buffer size of 22m) and a heuristic checking if 2/3 of buffers are near to a particular road.

The script can be parameterized with the following variables:

- `outfile`: file name to print results to (default is stdout)
- `buffer_size`: a float specifying the size/radius of the checkpoints (in meters, default `22.0`)
- `buffer_distance`: a float specifying the distance between checkpoints (in meters, default `100.0`)
- `paths_table`: the name of the table (or view) containing the ways that should be considered as "paths" (i.e., potential sidepaths, default `way_import_paths`)
- `roads_table`: the name of the table (or view) containing the ways that should be considered as "roads" (i.e., the roads that could have sidepaths, default `way_import_roads`)
- `format`: the output format. Choices are:
  - `is_sidepath_no`: single-column CSV with `osm_id`s of `paths_table` that *are not* estimated sidepaths (typically the smallest result)
  - `is_sidepath_yes`: single-column CSV with `osm_id`s of `paths_table` that *are* estimated sidepaths
  - `is_sidepath_csv`: two-column CSV with all `osm_id`s of `paths_table` and boolean `is_sidepath_estimation` indicating whether the path is an estimated sidepath or not 
  - `sidepath_dict`: an intermediate result of the sidepath estimation (called `sidepath_dict` in the original implementation) as lines of json (jsonl); mainly for debugging and exploration purposes.  

Example run:

```bash
psql postgresql://postgres:postgres@127.0.0.1:5432/postgres -1 -f ./generate_sidepath_dict.script.sql \
  -v outfile=sidepath_dict.jsonl \
  -v roads_table=cqi_roads \
  -v paths_table=cqi_paths \
  -v format=is_sidepath_no
```

## Output format for `sidepath_dict`

Each line is a json-array with 2 elements:

1. the id of the path, an integer
2. an object with the following fields:
   - `checks`: the number of checkpoints of the path 
   - `id`: an object listing the number of checkpoints that are *near* to a particular way-id of a road 
   - `highway`: an object listing the checkpoint count similarly to (3), but distinguished by highway type instead of way-id 
   - `name`: an object listing the checkpoint count similarly to (3), but distinguished by road name instead of way-id 
   - `maxspeed`: an object listing the maximum speed of the *nearby* roads, by highway type 

(Here, *near* means that the checkpoint has a distance lesser than or equal to `buffer_size` of a road)

Example line:

```json
[1256331095, {"id": {"19800045": 2, "24980582": 2, "184899566": 1, "836498473": 1, "1256008946": 2}, "name": {"Oberlandstraße": 2, "Schaffhausener Straße": 2}, "checks": 2, "highway": {"tertiary": 2, "residential": 2}, "maxspeed": {"tertiary": 50, "residential": 30}}]
```

## Roadmap/TODOs

- [ ] fix remaining differences to the original *sidepath_dict* calculation
- [ ] sql functions to distinguish paths and roads and see how much slower it is separating them on-the-fly 
