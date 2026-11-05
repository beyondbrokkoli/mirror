-- EXPECT: 1
-- EXPECT: x
local t = {{1}, {"x"}}
local row = t[0]
local cell = t[1][0]
t = nil
print(row[0])
print(cell)
