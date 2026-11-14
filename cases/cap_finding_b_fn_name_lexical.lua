-- EXPECT: 1
-- EXPECT: 1
-- Closure Finding B's SAFE half: fn-NAME resolution does not ride the
-- caller's frames. resolve_fn_binding cuts the scan at the innermost
-- expansion's frame base, falls back to the literal's definition-site
-- chain (fn_def_sites), then to the depth-0 projection (snap.fns) —
-- so M's same-name G can never re-point F's call. This is the tier
-- the Finding B fix must NOT disturb: data names resolve dynamically
-- at expansion while fn names already resolve at the definition.
-- LuaJIT: 1, 1
local function G() return 1 end
local function F() return G() end
local function M()
    local function G() return 1000 end
    return F()
end
print(M())
print(G())
