M = {}
--------------------------
-- utiltities
--------------------------
--
-- TODO: can we better type `fields`

--- remove any fields that are in the set of `unwanted_fields`
---@param object table
---@param unwanted_fields table a set of fields
---@return table
function M.with_fields_stripped(object, unwanted_fields)
  local result = {}
  for k, v in pairs(object) do 
    if not unwanted_fields[k] then
      result[k] = v
    end
  end
  return result
end

function M.join(...)
  local dest = {}
  for _, set in ipairs({...}) do
    for k, _ in pairs(set) do dest[k] = true end
  end
  return dest
end

---@param varname string
function M.getenv_required(varname)
  local r = os.getenv(varname)
  assert(r, string.format('required environment variable "%s" not found', varname))
  return r
end

return M

