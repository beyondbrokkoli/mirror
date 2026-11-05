-- EXPECT: 1
-- EXPECT: 3
-- EXPECT: 9
local f = function() return {1, 2} end
print(f()[0])
print(({3, 4})[0])
local g = function(x) print(x[0]) end
g({9, 8})
