-- EXPECT: 42
-- The empty-face-first memo env. A zero-capture literal's creation
-- read mints the statement's env first; a face-bearing creation read
-- in the SAME statement hits the memo vacuously (the shared names all
-- agree) and stores its cell into it. The env must mint with
-- contains_tables so free_tbl's child walk composts the cell — the
-- 0x01 variant stranded it at exit (leak_contract/p7). Judge: 42.
local n = 41
local function a() end
local function b() n = n + 1 end
local h1, h2 = a, b
h2()
print(n)
