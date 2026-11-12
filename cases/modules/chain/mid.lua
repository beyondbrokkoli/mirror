-- the middle of the DAG: requires the base, is required by the top
local Base = require("chain.base")
local Mid = {}
function Mid.pass(x) return Base.id(x) + 1 end
return Mid
