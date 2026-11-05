-- EXPECT: 0	1
-- EXPECT: done
-- EXPECT: 0	1	2	7
-- EXPECT: 0	99999	4

local t = {}
t[0] = 1
local i = -3
while i < 2 do
  t[i] = i
  i = i + 1
end
print(t[0], t[1])

local u = {}
u[0] = 1
local j = 0
while j < -5 do
  u[j] = j
  j = j + 1
end
print("done")

local m = {}
m[200000] = 7
local k = 0
while k < 3 do
  m[k] = k
  k = k + 1
end
print(m[0], m[1], m[2], m[200000])

local big = {}
local n = 0
while n < 100000 do
  big[n] = n
  n = n + 1
end
print(big[0], big[99999], sys_alloc_count())
