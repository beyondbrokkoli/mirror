-- EXPECT: 7	8
-- EXPECT: 0
do
  local m = {}
  m[0] = {7, 8}
  print(m[0][0], m[0][1])
end
print(sys_alloc_count())
