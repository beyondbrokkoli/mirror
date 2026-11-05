-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 4
-- EXPECT: 1
-- EXPECT: 3
-- EXPECT: 3
-- EXPECT: 3
-- EXPECT: 1
local mode = ({0})[0]
if mode == 1 then
    local q = {7, 8}
    print(q[0])
    return q
end
print(sys_alloc_count())
local other = {9}
if mode == 2 then return end
print(sys_alloc_count())
local f = function(c)
    local m = {3}
    if c == 1 then return m end
    print(m[0])
    return m
end
local g = function() return ({4})[0] end
print(g())
print(sys_alloc_count())
local r = f(0)
print(r[0])
r = nil
local s = f(1)
print(s[0])
s = nil
print(sys_alloc_count())
