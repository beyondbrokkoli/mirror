-- -- ARGS: 1 500 10

local mode = arg[0]
local seed = arg[1]
local limit = arg[2]
if mode > 0 then
    local state = { cursor = seed, emitted = 0, energy = 0 }
    local dyn_key = "cursor"
    for step = 1, 10 do
        if state.emitted >= limit then
            break
        end
        local n = state[dyn_key]
        state.energy = state.energy + n
        state.emitted = state.emitted + 1
    end
    state = nil
end
return arg
