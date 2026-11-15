-- EXPECT: 41
-- Phase 4 cell-backed locals, triage Bug 4 — LANDED GREEN. The face's
-- type is stamped depth-aware (var_type_at_depth): the Str shadow in
-- the do-block cannot split the face from Make's Int n, so the
-- judge-legal program compiles and returns 41. Was a loud false
-- reject (the Closure face-Ty law — retired with §4's unification).
local function Make()
    local n = 41
    local function F() return n end
    do
        local n = "hi"
        local f = F
        return f()
    end
end
print(Make())
