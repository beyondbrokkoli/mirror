-- EXPECT: 4
-- EXPECT: 0
-- EXPECT: 6
-- EXPECT: 0

do
  local x = {}
  x[0] = 3
  local y = {}
  y[0] = 4
  if 2 < 1 then
    y = x
  end
  print(y[0])
end
print(sys_alloc_count())
do
  local b = {}
  b[0] = 6
  local c = {}
  if 1 < 2 then
    c = b
  end
  print(c[0])
end
print(sys_alloc_count())
