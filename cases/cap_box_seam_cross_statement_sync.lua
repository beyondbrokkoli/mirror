-- EXPECT: 41
-- EXPECT: 100
-- EXPECT: 100
-- Cross-statement cell sync: two creation reads of the same
-- closure in DIFFERENT statements mint two envs over one binding.
-- Pre-fix each mint snapshotted its own cell (the second creation
-- never heard the definer's write). The one-box registry keys the
-- cell on the binding's slot identity, so both closures share it.
-- LuaJIT: 41, 100, 100.
local n = 41
local function Get() return n end
local a = Get
local b = Get
print(a())
n = 100
print(b())
print(Get())
