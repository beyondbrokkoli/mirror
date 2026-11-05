-- EXPECT: 7
-- EXPECT: 9
local Camera = {}
function Camera.new(x) return x end
print(Camera.new(7))
function Deep.Nest.f(x) return x + 2 end
print(Deep.Nest.f(7))
