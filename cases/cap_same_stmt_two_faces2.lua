-- Closure Finding A pin: two closure creation reads in ONE statement
-- whose same-name captures resolve to DIFFERENT bindings (the inner
-- do-block shadows the outer x). The per-statement env memo used to
-- share one env and skip already-stored names by string, so the
-- second face was silently absorbed: alm printed 10, 10. The memo now
-- keys sharing on the capture's binding depth, and the cell store
-- resolves at that depth — two envs, two cells, each face its own.
-- LuaJIT: 10, 1
-- EXPECT: 10
-- EXPECT: 1
local function Make()
    local x = 1
    local function GetOuter() return x end
    local rec
    do
        local x = 10
        local function GetInner() return x end
        rec = { a = GetInner, b = GetOuter }
    end
    return rec
end
local d = Make()
print(d.a())
print(d.b())
