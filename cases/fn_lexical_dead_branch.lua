-- EXPECT: 1
-- EXPECT: 0
local f = function() return 1 end
local x = 0
if 1 < 0 then
    f = function() return 2 end
    x = 5
end
print(f())
print(x)
