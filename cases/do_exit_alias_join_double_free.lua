-- EXPECT: 0

do
  local w = {}
  w[0] = 5
  local pick = {}
  if 1 > 2 then
    pick = w
  end
end
print(sys_alloc_count())
