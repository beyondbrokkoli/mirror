-- EXPECT: 99

local t = {{99}}
local row = t[0]
t = nil
print(row[0])
