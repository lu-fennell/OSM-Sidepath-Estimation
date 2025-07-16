set dotenv-load := true

# TODO: requirements: curl, osm2pgsql, psql, just
pg-user := env("PGUSER")
pg-pass := env("PGPASSWORD")
pg-database := env("PGDATABASE")

pg-host := env("PGHOST", "127.0.0.1")
pg-url := 'postgres://' + pg-user + ':' + pg-pass + '@' + pg-host + '/' + pg-database

outdir := 'output'

osm-download-dir := 'osmfiles'

today := shell('date -I')
default-paths-table := '_sidepath_estimation_paths'
default-roads-table := '_sidepath_estimation_roads'

sidepath-estimation-pipelines-all-maps date=today format='is_sidepath_no' force-download='no':
  for map in europe/germany/berlin europe/germany/brandenburg europe/germany; do \
    just sidepath-estimation-pipeline "$map" "{{date}}" "{{force-download}}" "{{format}}"; \
  done


sidepath-estimation-pipeline map-name date force-download +formats: 
  #!/usr/bin/env bash
  source just.sh
  just download-pbf "{{ map-name }}" "{{ date }}" "{{ force-download }}"
  MAP_FILE_STEM="`map-file-stem {{map-name}}`"
  DEST="`map-file-dest "{{ osm-download-dir }}" "{{ map-name }}" "{{ date }}"`"
  PATHS_TABLE_NAME="spe_paths_${MAP_FILE_STEM}"
  ROADS_TABLE_NAME="spe_roads_${MAP_FILE_STEM}"
  just osm2pgsql "${DEST}" "${PATHS_TABLE_NAME}" "${ROADS_TABLE_NAME}"
  for format in {{formats}}; do
    CSV_FILE="`csv-file "{{ outdir }}" "{{ map-name }}" "$format" "{{ date }}"`"
    just sidepath-estimation \
      "${CSV_FILE}" \
      "$format"  \
      "${PATHS_TABLE_NAME}" \
      "${ROADS_TABLE_NAME}"
    just update-latest "{{ map-name }}" "$format" "{{ date }}" 
  done

osm2pgsql osm-file $PATHS_TABLE_NAME=default-paths-table $ROADS_TABLE_NAME=default-roads-table:
  osm2pgsql -O flex -S osm2pgsql/sideway_estimation_roads_and_paths.lua -d "{{ pg-url }}" "{{osm-file}}"

sidepath-estimation outfile format paths-table=default-paths-table roads-table=default-roads-table: 
  time psql -h "{{ pg-host }}" -f sql/generate_sidepath_estimation.script.sql \
    -v format="{{ format }}" \
    -v outfile="{{ outfile }}" \
    -v paths_table="{{ paths-table }}" \
    -v roads_table="{{ roads-table }}" 

download-pbf map-name date=today force-download='no':
  #!/usr/bin/bash
  source just.sh
  mkdir -p {{ osm-download-dir }} 
  MAP_FILE_STEM="`map-file-stem {{map-name}}`"
  DEST="`map-file-dest "{{ osm-download-dir }}" "{{ map-name }}" "{{ date }}"`"
  if [ "{{ date }}" = "{{ today }}" ]; then
    just download-latest-pbf-to "{{ map-name }}" "$DEST" "{{ force-download }}"
  elif [ ! -f "${DEST}" ]; then
    echo "ERROR: map for date '{{ date }}' has not been downloaded to '${DEST}'. We can only download maps from today ({{ today }})"
  fi

download-latest-pbf-to map-name outfile force='no':
  #!/usr/bin/env bash
  set -euo pipefail
  URL="https://download.geofabrik.de/{{ map-name }}-latest.osm.pbf"
  if [ "{{ force }}" = "yes" -o ! -f "{{ outfile }}" ]; then
    echo downloading "$URL -> {{ outfile }}"
    curl -L "$URL" -o "{{ outfile }}"
  else
    echo "outfile '{{ outfile }}' already exists. Use 'force' to overwrite"
  fi

update-latest map-name format='is_sidepath_no' date=today:
  #!/usr/bin/env bash
  source just.sh
  CSV_FILE="`csv-file "{{ outdir }}" "{{ map-name }}" "{{ format }}" "{{ date }}"`"
  cp -v "$CSV_FILE" "${CSV_FILE/{{ date }}/latest}"
  

psql *args:
  psql -h {{ pg-host }} {{ args }}


test *args:
  PGURL="{{pg-url}}" cargo test {{ args }}
