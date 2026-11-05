-- EXPECT: 1536
-- EXPECT: -1536
-- EXPECT: 1
-- EXPECT: -0.5
-- EXPECT: 2048
-- EXPECT: -1
-- EXPECT: -2
-- EXPECT: 1048576
-- EXPECT: 3072
-- EXPECT: 512
local Fixed = {}
local SCALE = 1024

function Fixed.from_float(v)
    return (v * 1024.0) // 1.0
end

function Fixed.to_float(v)
    return v / SCALE
end

function Fixed.mul(a, b)
    return (a * b) // SCALE
end

function Fixed.div(a, b)
    return (a * SCALE) // b
end

function Fixed.add(a, b)
    return a + b
end

function Fixed.sub(a, b)
    return a - b
end

print(Fixed.from_float(1.5))
print(Fixed.from_float(-1.5))
print(Fixed.to_float(1024))
print(Fixed.to_float(-512))
print(Fixed.mul(1024, 2048))
print(Fixed.mul(-1, 1))
print(Fixed.div(-3, 2048))
print(Fixed.div(3072, 3))
print(Fixed.add(1024, 2048))
print(Fixed.sub(1024, 512))
