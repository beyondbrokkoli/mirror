-- EXPECT: 2
local f = function() return 1 end
do
  f = function() return 2 end
end
print(f())
