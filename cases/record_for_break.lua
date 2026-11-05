-- -- ARGS: 1 500 10

local mode = arg[0]
local seed = arg[1]
local limit = arg[2]
if mode > 0 then
    local state = { cursor = seed, emitted = 0, energy = 0 }
    for step = 1, 10 do
        if state.emitted >= limit then
            break
        end
        state.energy = state.energy + seed
        state.emitted = state.emitted + 1
    end
    state = nil
end
return arg
