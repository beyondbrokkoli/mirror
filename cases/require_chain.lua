-- the engine's layering shape: a three-module DAG through the root,
-- with a shared floor reached from two directions (mid and main).
-- FLAGS: --module-root=cases/modules
-- EXPECT: base: floor
-- EXPECT: 8
-- EXPECT: 41
local Mid = require("chain.mid")
local Base = require("chain.base")
print(Mid.pass(7))
print(Base.id(41))
