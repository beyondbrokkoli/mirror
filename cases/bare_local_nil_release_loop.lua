-- EXPECT: 0

do
  local x
  local i = 0
  while i < 2 do
    x = nil
    i = i + 1
  end
  x = nil
end
print(sys_alloc_count())
