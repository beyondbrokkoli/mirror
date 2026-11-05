-- EXPECT: 0
-- EXPECT: 2
-- EXPECT: 2.5
-- EXPECT: 4.5

do
  local temp
  local i = 0
  while i < 2 do
    temp = i * 2
    print(temp)
    i = i + 1
  end
end
do
  local temp
  local fs = {}
  fs[0] = 2.5
  fs[1] = 4.5
  local i = 0
  while i < 2 do
    temp = fs[i]
    print(temp)
    i = i + 1
  end
end
