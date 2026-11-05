-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 20
-- EXPECT: 0
-- EXPECT: 0
do
  local t = {1, 2}
end
print(sys_alloc_count())
do
  local t = { {1}, {2} }
end
print(sys_alloc_count())
do
  local t = {[0] = {10, 20}, [1] = {30, 40}}
  print(t[0][1])
end
print(sys_alloc_count())
do
  local t = {1}
  t = nil
end
print(sys_alloc_count())
