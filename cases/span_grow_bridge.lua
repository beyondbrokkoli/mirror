
-- EXPECT: 42
do
local t = {}
local i = 0
while i < 1000 do
    t[i] = i
    i = i + 1
end
local idx = 1
local j = 0
while j < 63 do
    idx = idx * 2
    j = j + 1
end
idx = idx - 1
t[idx] = 42
print(t[idx])
end

-- EXPECT: 40
do
local t = {}
local i = 0
while i < 1000 do
    t[i] = i
    i = i + 1
end

local idx = 1
local j = 0
while j < 63 do
    idx = idx * 2
    j = j + 1
end
idx = idx - 1

t[idx] = 40
print(t[idx])
end

-- EXPECT: If you see this, the bullet missed the vital organs.
do
local t = {}

t[0] = 1

local idx = 1
local j = 0
while j < 63 do
    idx = idx * 2
    j = j + 1
end
idx = idx - 1

t[idx] = 1099511627776

t[10] = 42

print("If you see this, the bullet missed the vital organs.")
end
