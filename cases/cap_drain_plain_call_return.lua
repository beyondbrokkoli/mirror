-- EXPECT: 5
-- The plain inlined call whose body ends in `return`. The body
-- scope's pop free rides the dead continuation after the return
-- terminated the block; the return edge must carry the crossed
-- scope's unheld cell itself (leak_contract/p4a — the IR showed the
-- alm_tbl_free in a predecessor-less block). Judge: 5.
local function make()
  local x = 5
  local g = function() return x end
  return g()
end
print(make())
