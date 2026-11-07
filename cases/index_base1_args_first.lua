-- FLAGS: --index-base=1
-- ARGS: 2
-- EXPECT: 1
-- EXPECT: 3
-- EXPECT: 20
-- The boundary arg table is physical on the host side and the seam
-- translates its reads: arg[1] is the first runtime argument (value
-- 2). The positional constructor's user key 2 is its second element
-- under base 1 (the third under base 0) -- the same program with
-- arg[0]/key 2 at base 0 reads 30, so this pin is the base-1 face of
-- args_key_position.
local t = {}
t[arg[1]] = 1
print(t[2])
local u = {[arg[1]] = 3}
print(u[2])
print({10, 20, 30}[arg[1]])
