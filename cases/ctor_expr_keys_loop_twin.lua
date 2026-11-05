-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 10
-- EXPECT: 10
-- EXPECT: 20
-- EXPECT: 20
local i = 0
while i < 3 do
  local row = {[i] = i * 10}
  print(row[i])
  local twin = {}
  twin[i] = i * 10
  print(twin[i])
  i = i + 1
end
