
-- EXPECT: survived double nil
do
local t = {}
t[0] = 41
t = nil
t = nil
print("survived double nil")
end

-- EXPECT: loop drop ok
do
local t = {}
t[0] = 41
local i = 0
while i < 3 do
    t = nil
    i = i + 1
end
print("loop drop ok")
end

-- EXPECT: 7
-- EXPECT: 9
do
local t = {}
t[0] = 7
local u = t
t = nil
print(u[0])
u = nil
t = {}
t[1] = 9
print(t[1])
end

-- EXPECT: 5
-- EXPECT: 0
do
local a = {}
a[0] = 1
local b = {}
b[0] = 2
local pick = a
if 1 > 2 then
    pick = b
end
pick[0] = 5
print(pick[0])
pick = nil
end
print(sys_alloc_count())

-- EXPECT: 0
-- EXPECT: eager
do
local a = {}
a[0] = 1
local target
target = a
target = nil
print(sys_alloc_count())
print("eager")
end

-- EXPECT: 0
-- EXPECT: dropped
do
local a = {}
a[0] = 1
local b = {}
b[0] = 2
local target = a
if false then
    target = b
end
target[0] = 5
target = nil
end
print(sys_alloc_count())
print("dropped")
