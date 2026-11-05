-- EXPECT: 10	20	30
-- EXPECT: 1.5	2.5	0
-- EXPECT: hello	world
-- EXPECT: true	false	false
-- EXPECT: 9
-- EXPECT: 1	2
do
local a = {10, 20, 30}
print(a[0], a[1], a[2])
local f = {1.5, 2.5}
print(f[0], f[1], f[2])
local s = {"hello", "world"}
print(s[0], s[1])
local b = {true, false}
print(b[0], b[1], b[2])
local e = {}
e[0] = 9
print(e[0])
local trailing = {1, 2,}
print(trailing[0], trailing[1])
end
