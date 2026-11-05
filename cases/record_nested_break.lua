-- -- ARGS: 1

local mode = arg[0]
local seed = arg[1]
local limit = arg[2]
if mode > 0 then
    local state = { cursor = seed, emitted = 0, energy = 0 }
    for step = 1, 10 do
        if state.emitted >= limit then
            break
        end
        local n = state.cursor
        for div = 3, 5 do
            if div > n then
                break
            end
            if n % div == 0 then
                break
            end
        end
        state.energy = state.energy + n
        state.emitted = state.emitted + 1
        state.cursor = n + 1
    end
    state = nil
end
return arg
