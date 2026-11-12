-- the cycle's first leg
local Beta = require("cycle.beta")
local Alpha = {}
function Alpha.via(x) return Beta.twice(x) end
return Alpha
