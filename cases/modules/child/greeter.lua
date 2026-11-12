-- the child module: top-level side effect + a returned record with a
-- closure field — the two-module acceptance shape
print("child: top-level runs")
local Greeter = {}
function Greeter.hello(name) return "hello " .. name end
return Greeter
