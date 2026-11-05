-- EXPECT: 0

do
  local x
  if 1 < 2 then
    local w = {}
    w[0] = 7
    x = w
  end
end
print(sys_alloc_count())
