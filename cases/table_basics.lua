
-- EXPECT: 1.5	2.5	0
-- EXPECT: 0	7.25
-- EXPECT: 1.125	2.5
-- EXPECT: 3.5	5.5	0.5
do
local t = {}
t[0] = 1.5
t[1] = 2.5
print(t[0], t[1], t[2])

t[10] = 7.25
print(t[5], t[10])

local u = t
u[20] = 1.125
print(u[20], u[1])

local i = 0
while i < 24 do
    u[i] = u[i] * 2.0 + 0.5
    i = i + 1
end
print(u[0], u[1], u[23])
u = nil
end


-- EXPECT: true	false	false
-- EXPECT: false	true
do
local t = {}
t[0] = true
t[1] = false
print(t[0], t[1], t[2])
local i = 0
while i < 5 do
    t[i] = not t[i]
    i = i + 1
end
print(t[0], t[3])
end

-- EXPECT: hello	world
-- EXPECT: world
do
local s = {}
s[0] = "hello"
s[1] = "world"
print(s[0], s[1])
local u = s
print(u[1])
end

-- EXPECT: 1.5	2.5
do
local t = {}
t[0] = 1.5
local u = t
u[1] = 2.5
print(u[0], u[1])
end

-- EXPECT: true
-- EXPECT: false
do
local a = {}
print(a == a)
print(a == {})
end

-- EXPECT: 1
-- EXPECT: 10
-- EXPECT: 2
do
local t = {}
t[0] = 1
t[1] = 2
t[2] = 3
print(t[0])
do
  local big = {}
  big[0] = 10
  big[1] = 20
  big[2] = 30
  big[3] = 40
  big[4] = 50
  print(big[0])
end
print(t[1])
end
