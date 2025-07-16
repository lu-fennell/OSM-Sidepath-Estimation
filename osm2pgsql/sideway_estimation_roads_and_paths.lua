package.path = package.path .. ";./osm2pgsql/?.lua"
local util = require('util')
require('HighwayClasses')

local roadsTable = osm2pgsql.define_table({
  name = util.getenv_required('ROADS_TABLE_NAME'),
  ids = { type = 'any', id_column = 'id', type_column = 'osm_type' },
  columns = {
    { column = 'tags',    type = 'jsonb' },
    { column = 'geom',    type = 'linestring' },
  },
  indexes = {
    { column = 'geom', method = 'gist' },
    { column = 'id',   method = 'btree', unique = true }
  }
})


local pathsTable = osm2pgsql.define_table({
  name = util.getenv_required('PATHS_TABLE_NAME'),
  ids = { type = 'any', id_column = 'id', type_column = 'osm_type' },
  columns = {
    { column = 'tags',    type = 'jsonb' },
    { column = 'geom',    type = 'linestring' },
  },
  indexes = {
    { column = 'geom', method = 'gist' },
    { column = 'id',   method = 'btree', unique = true }
  }
})

function osm2pgsql.process_way(object)

  local road_highway_classes = util.join(
    HighwayClasses,
    MajorRoadClasses,
    MinorRoadClasses
  )
  local path_highway_classes = PathClasses
  
  local tags = object.tags

  -- ====== (A) Filter-Guards ======
  if not tags.highway then return end

  -- TODO: do I still want/need this?
  -- Skip ways that are not relevant for sidepath estimation
  -- if not IsSidepathRelevant(tags) then return end
   
  -- Skip any area. See https://github.com/FixMyBerlin/private-issues/issues/1038 for more.
  if tags.area == 'yes' then return end


  -- ====== (B) Compute results and insert ======


  local insert_table = {
    tags = util.with_fields_stripped(object, Set({'nodes'})),
    geom = object:as_linestring(),
  }

  if road_highway_classes[object.tags.highway] then
    roadsTable:insert(insert_table)
  elseif path_highway_classes[object.tags.highway] then
    pathsTable:insert(insert_table)
  end
end

