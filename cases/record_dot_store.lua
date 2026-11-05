-- -- ARGS: 1 500 10

local mode = arg[0]
local seed = arg[1]
local limit = arg[2]
if mode > 0 then
    local state = { cursor = seed, emitted = 0, energy = 0 }
    state.emitted = state.emitted + 1
    state.energy = state.energy + seed
    state = nil
end
return arg
