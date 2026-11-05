-- EXPECT: 21
-- EXPECT: 1
-- EXPECT: 2
-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 4
-- EXPECT: 4
-- EXPECT: 1
-- EXPECT: 7
-- EXPECT: 3
-- EXPECT: 1
-- EXPECT: 5
-- EXPECT: 3
-- EXPECT: 5
-- EXPECT: 1
-- EXPECT: 9
-- EXPECT: 4
-- EXPECT: 9
-- EXPECT: 1
-- ARGS: 1
local g = function(c)
    local a = {{21, 22}, {23}}
    return a[0]
end
local r = g(0)
print(r[0])
r = nil
print(sys_alloc_count())
r = g(0)
r = {77}
print(sys_alloc_count())
r = nil
print(sys_alloc_count())
local base = {{1, 2}, {3}}
do
    local inner = base[0]
    print(inner[0])
end
print(base[0][0])
print(sys_alloc_count())
local sib = base[1]
sib = nil
print(sys_alloc_count())
base = nil
print(sys_alloc_count())
local deep = function(c)
    local a = {{{7, 8}}}
    local b = a[0]
    return b
end
local parent = deep(0)
local child = parent[0]
print(child[0])
child = nil
print(sys_alloc_count())
parent = nil
print(sys_alloc_count())
local x = {{5, 6}}
local id = function(p) return p end
local pt = id(x[0])
print(pt[0])
pt = nil
print(sys_alloc_count())
print(x[0][0])
x = nil
print(sys_alloc_count())
local mix_base = {{9, 8}, {7}}
local mix
if arg[0] == 1 then mix = mix_base[0] else mix = {99} end
print(mix[0])
mix = nil
print(sys_alloc_count())
print(mix_base[0][0])
mix_base = nil
print(sys_alloc_count())
