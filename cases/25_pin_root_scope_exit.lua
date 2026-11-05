-- EXPECT: 5
-- EXPECT: 6
local flag = ({1})[0]
local t = {5, 6}
if flag == 0 then return t end
print(t[0])
print(t[1])
