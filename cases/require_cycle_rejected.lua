-- the DAG law one level up: a require cycle is the recursion
-- diagnostic's voice at the module tier, rejected by name.
-- FLAGS: --module-root=cases/modules
-- EXPECT_BUILD_FAIL: require cycle: cycle.alpha -> cycle.beta -> cycle.alpha
local Alpha = require("cycle.alpha")
print(Alpha.via(3))
