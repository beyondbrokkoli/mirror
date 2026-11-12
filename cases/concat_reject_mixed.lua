-- '..' concatenates String with String — the dialect has no coercion;
-- an Int on either side is a named rejection, spelled conversion is
-- the caller's job (until a conversion law exists).
-- EXPECT_BUILD_FAIL: '..' concatenates String with String
local n = 42
print(n .. "is the answer")
