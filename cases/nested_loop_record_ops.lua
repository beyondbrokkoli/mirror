-- -- ARGS: 1

local mode = arg[0]
if mode > 0 then
    local state = { a = 1, b = 2 }
    for i = 1, 10 do
        for j = 1, 5 do
            state.b = state.b + j
        end
        state.a = state.a + 1
    end
    state = nil
end
return arg
