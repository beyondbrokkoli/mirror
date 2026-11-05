-- EXPECT: 2
local f = function() return 1 end
local n = 0
while n > 0 do
    n = n - 1
    f = function() return 2 end
end
print(f())
