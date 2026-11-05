-- EXPECT: 4
-- EXPECT: 3
-- EXPECT: 0
-- EXPECT: 21
-- EXPECT: 0
local x = {{4, 5}, {6}}
print(x[0][0])
local mode = ({0})[0]
if mode == 1 then return x[0] end
print(sys_alloc_count())
x = nil
print(sys_alloc_count())
local g = function(c)
    local a = {{21, 22}, {23}}
    if c == 1 then return {99} end
    return a[0]
end
local r = g(0)
print(r[0])
r = nil
print(sys_alloc_count())
