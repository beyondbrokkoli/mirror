-- EXPECT: 110
-- EXPECT: 18
-- EXPECT: 0
local t = {}
for i = 0, 10 do
    t[i] = i * 2
end
local u = {}
for j = 0, 10 do
    if j == 5 then break end
    u[j] = j * 2
end
local s = 0
for k = 0, 10 do
    s = s + t[k] + u[k]
end
print(s)
print(t[9])
print(u[5])
