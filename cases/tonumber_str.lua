-- tonumber's digits-only parse: a literal folds at the lowerer (no
-- runtime call), a pooled handle rides alm_str_tonumber — both answer
-- Int.
-- EXPECT: 42
-- EXPECT: 7

local n = tonumber("42")
print(n)
local s = "3" .. "4" .. ""
print(tonumber(s) - 27)
