-- -- ARGS: 1

local mode = arg[0]
if mode > 0 then
    local state = { a = 1, b = 2 }
    for i = 1, 10 do
        local n = state.b
        if n > 0 then
            state.a = state.a + n
        end
        state.b = n + 1
    end
    state = nil
end
return arg
