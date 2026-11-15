-- EXPECT: 5
-- The early return inside the body: both return edges cross the body
-- scope, and each must drain the unheld cell it crosses
-- (leak_contract/p4). Judge: 5.
local function make()
  local x = 5
  local g = function() return x end
  if x == 5 then return g() end
  return 0
end
print(make())
