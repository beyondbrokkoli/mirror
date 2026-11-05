-- ARGS: 7
-- EXPECT: 7
-- EXPECT: 8
local pass = function(x) return x end
local y = pass(pass(arg[0]))
local z
if y > 2 then z = y else z = 0 end
print(z)
print(z + 1)
