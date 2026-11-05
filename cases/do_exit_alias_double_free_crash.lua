-- EXPECT: 1
-- EXPECT: 0
do
  local c = {}
  c[0] = 1
  local b = c
  print(sys_alloc_count())
end
print(sys_alloc_count())
