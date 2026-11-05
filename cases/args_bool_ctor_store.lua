-- ARGS: true
-- EXPECT: 1
-- EXPECT: true
local u = {arg[0]}
if u[0] then print(1) end
print(u[0])
