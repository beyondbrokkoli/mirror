-- EXPECT_BUILD_FAIL: Type Error: a closure record's non-closure fields are 8-byte scalars (Int/Flt/Str) — its element type is IntTable
local function Make()
    local n = 1
    local function Step() n = n + 1 return n end
    return { step = Step, kid = { [0] = 1 } }
end
local t = Make()
print(t.step())
