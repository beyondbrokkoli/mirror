-- EXPECT: 99
local Fixed = {}
function Fixed.add(a, b)
    return a + b
end
function Fixed.add(a, b)
    return a - b + 100
end
print(Fixed.add(10, 11))
