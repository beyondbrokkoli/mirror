-- EXPECT: 1	2
-- EXPECT: 10	20	40
-- EXPECT: 9
do
local b = {[0] = 1, [5] = 2}
print(b[0], b[5])
local c = {10, 20, [3] = 40}
print(c[0], c[1], c[3])
local sparse = {[100001] = 9}
print(sparse[100001])
end
