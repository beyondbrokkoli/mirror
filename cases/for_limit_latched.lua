-- EXPECT: 3
-- EXPECT: 1
-- EXPECT: 10
local n = 5
local trips = 0
for i = 0, n do
  n = 1
  trips = trips + 1
  if i == 2 then break end
end
print(trips)
print(n)
local c = 0
for i = 0, 10 do c = c + 1 end
print(c)
