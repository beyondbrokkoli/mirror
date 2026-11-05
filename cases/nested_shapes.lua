
-- EXPECT: 1
do
local t = {}
t[0] = {}
t[0][0] = 1
print(t[0][0])
end

-- EXPECT: 2
do
local inner = {}
inner[0] = 2
local t = {}
t[0] = inner
print(t[0][0])
end

-- EXPECT: 1
-- EXPECT: 2
-- EXPECT: 3
-- EXPECT: 4
do
local r0 = {}
r0[0] = 1
r0[1] = 2
local r1 = {}
r1[0] = 3
r1[1] = 4
local mat = {}
mat[0] = r0
mat[1] = r1
print(mat[0][0])
print(mat[0][1])
print(mat[1][0])
print(mat[1][1])
end

-- EXPECT: 42
-- EXPECT: 2
-- EXPECT: 3
-- EXPECT: 4
do
local r0 = {}
r0[0] = 1
r0[1] = 2
local r1 = {}
r1[0] = 3
r1[1] = 4
local m = {}
m[0] = r0
m[1] = r1
m[0][0] = 42
print(m[0][0])
print(m[0][1])
print(m[1][0])
print(m[1][1])
end
