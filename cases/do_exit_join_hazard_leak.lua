-- EXPECT: 4
-- EXPECT: 0

do
  local pick = {}
  pick[0] = 3
  if 1 < 2 then
    local w = {}
    w[0] = 4
    pick = w
  end
  print(pick[0])
end
print(sys_alloc_count())
