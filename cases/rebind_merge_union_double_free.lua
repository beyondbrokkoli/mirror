-- EXPECT: 1
-- EXPECT: 0
-- EXPECT: 0

do
  local w = {3, 4}
  local x = {1, 2}
  if 1 < 2 then
    x = w
  end
  print(sys_alloc_count())
end
print(sys_alloc_count())
do
  local t = {1, 2}
  local i = 0
  while i < 3 do
    t = {}
    t[0] = i
    i = i + 1
  end
end
print(sys_alloc_count())
