-- EXPECT: 60000
-- EXPECT: 3	1	3.5	-4	2

local t = {}
local lim = 60000
local outer = 0
while outer < lim do
    local inner = outer
    while inner < lim do
        t[inner] = inner + 1
        inner = inner + 1
    end
    outer = outer + 1
end
print(t[lim - 1])

print(7 // 2, 7 % 3, 7 / 2, -7 // 2, -7 % 3)
