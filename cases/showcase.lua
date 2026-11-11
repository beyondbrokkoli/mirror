
-- EXPECT: === 1. Basic Types & Strict Arithmetic ===
-- EXPECT: Int Ops:	22	12	85	3.4	3	2
-- EXPECT: Float Ops:	12.56636
-- EXPECT: === 2. Control Flow & Logic ===
-- EXPECT: Mild
-- EXPECT: Logic:	false	true	true
-- EXPECT: === 3. Integer Tables ===
-- EXPECT: Elements:	0	100	200	300	400
-- EXPECT: === 4. Float Tables ===
-- EXPECT: Temps:	98.6	98.6	98.6
-- EXPECT: === 5. Boolean Tables ===
-- EXPECT: Flags:	true	true	true	true
-- EXPECT: === 6. String Tables ===
-- EXPECT: Names:	ALM	Compiler	Rust
-- EXPECT: === 7. Lifetime Management ===
-- EXPECT: Cache hit:	42
-- EXPECT: Cache released safely
-- EXPECT: === Showcase Complete ===

print("=== 1. Basic Types & Strict Arithmetic ===")
local a = 17
local b = 5
print("Int Ops:", a + b, a - b, a * b, a / b, a // b, a % b)

local pi = 3.14159
local r = 2.0
local area = pi * (r * r)
print("Float Ops:", area)

print("=== 2. Control Flow & Logic ===")
local temp = 25
if temp > 30 then
    print("Hot")
elseif temp < 10 then
    print("Cold")
else
    print("Mild")
end

local is_ready = true
local has_key = false
print("Logic:", is_ready and has_key, is_ready or has_key, not has_key)

print("=== 3. Integer Tables ===")
local scores = {}
local i = 0
while i < 5 do
    scores[i] = i * 100
    i = i + 1
end
print("Elements:", scores[0], scores[1], scores[2], scores[3], scores[4])

print("=== 4. Float Tables ===")
local temps = {}
local t = 0
while t < 3 do
    temps[t] = 98.6
    t = t + 1
end
print("Temps:", temps[0], temps[1], temps[2])

print("=== 5. Boolean Tables ===")
local flags = {}
local f = 0
while f < 4 do
    flags[f] = true
    f = f + 1
end
print("Flags:", flags[0], flags[1], flags[2], flags[3])

print("=== 6. String Tables ===")
local names = {}
names[0] = "ALM"
names[1] = "Compiler"
names[2] = "Rust"
print("Names:", names[0], names[1], names[2])

print("=== 7. Lifetime Management ===")
local cache = {}
cache[0] = 42
print("Cache hit:", cache[0])

cache = nil
print("Cache released safely")

print("=== Showcase Complete ===")
