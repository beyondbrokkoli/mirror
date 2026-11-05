-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 0


local c0 = 0
local c1 = 1

do
  local a = {}
  a[0] = 5
  if c0 == 1 then
    a = {9}
  end
  local b = {}
  if c0 == 1 then b = a else b = a end
end
print(sys_alloc_count())

do
  local a = {}
  a[0] = 5
  if c1 == 1 then
    a = {9}
  end
  local b = {}
  if c1 == 1 then b = a else b = a end
end
print(sys_alloc_count())

do
  local a = {}
  a[0] = 5
  if c1 == 1 then
    a = {9}
  end
  local b = {}
  b[0] = 6
  local c = {}
  if c1 == 1 then
    c = b
  end
end
print(sys_alloc_count())
