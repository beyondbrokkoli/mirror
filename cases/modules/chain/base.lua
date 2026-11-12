-- the chain's floor: expanded first (nested-first), hoisted to the
-- root preamble ahead of every main statement
print("base: floor")
local Base = {}
function Base.id(x) return x end
return Base
