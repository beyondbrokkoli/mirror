-- FLAGS: --index-base=1
-- EXPECT: 2
-- EXPECT: 2
-- XFAIL: judge_divergence_audit/mint-pointer — the Tbl face stores
-- the mint-time pointer, so the direct surface reads the stale table
-- after the defining-scope rebind; the judge's upvalue rides the
-- binding and prints 2 twice. Flip when the env rides the binding —
-- the leak_contract/p9 alias-by-pointer decision reversed.
-- The captured-table mint-pointer law (pinned by decision
-- 2026-11-15, leak_contract/p9): a Tbl face aliases by pointer — no
-- box rides it — so the env stores the mint-time table pointer, and
-- a defining-scope rebind AFTER a creation read is seen by the
-- direct surface (the view redirects to the owner slot) but not by
-- value-surface calls, which read the env's pointer. The judge
-- (luajit upvalue semantics) would print 2 twice; the dialect's law
-- is alias-by-pointer, and the rebind-before-first-creation-read or
-- cell mutation are the sanctioned styles. The 2026-11-15 pin blesses
-- the divergence; the xfail pin below tracks the judge's answer
-- (2 twice) instead — the env riding the binding is the owed fix.
local t = {1}
local g = function() print(t[1]) end
local h = g
t = {2}
h()
g()
