-- EXPECT: 7	8
-- EXPECT: 0
-- EXPECT: 3
-- EXPECT: 0
do
  local k = 1
  local t = {[k] = {7, 8}}
  print(t[k][0], t[k][1])
end
print(sys_alloc_count())
do
  local k = 0
  local t = {[k] = {}}
  t[k][0] = 3
  print(t[0][0])
end
print(sys_alloc_count())
