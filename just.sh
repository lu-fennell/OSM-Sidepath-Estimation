# utility functions, to be sourced by just commands
#

set -euo pipefail

function map-file-stem {
  echo "${1}" | tr / _
}

function map-file-dest {
  echo "${1}/`map-file-stem "$2"`-$3.osm.pbf"
}

function csv-file {
  local outdir="$1"
  local mapname="$2"
  local format="$3"
  local date="$4"
  echo "${outdir}/$format-`map-file-stem "$mapname"`-$date.csv"
}
