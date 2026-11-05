-- EXPECT: 9
-- EXPECT: 0

do
  local x = {}
  x[0] = 9
  local m = {x, {1}}
  print(m[0][0])
end
print(sys_alloc_count())
