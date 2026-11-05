-- EXPECT: 0

do
  local t = {}
  t[0] = 5
  local i = 0
  while i < 1 do
    t = nil
    i = i + 1
  end
  t = nil
end
print(sys_alloc_count())
