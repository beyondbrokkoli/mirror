-- EXPECT: 7
local f = function()
    while true do break end
    return 7
end
print(f())
