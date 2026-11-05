-- ARGS: world
-- EXPECT: world
local echo = function(x) print(x) end
echo(arg[0])
