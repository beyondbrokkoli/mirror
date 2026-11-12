-- '..' is the string lane: Str .. Str -> Str, interned through the
-- pool so the product is pointer-identical to any literal with the
-- same bytes.
-- EXPECT: ab
-- EXPECT: alpha*
local a = "a"
local b = "b"
print(a .. b)
local base = "alpha"
print(base .. "*")
