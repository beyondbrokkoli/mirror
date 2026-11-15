-- EXPECT: 42
-- EXPECT: 42
-- Phase 4 cell-backed locals, triage Bug 1 — LANDED GREEN. A direct
-- F() call inside g's through-value expansion: g's face local binds
-- THE cell (no box[0] snapshot), F's direct-surface view rides the
-- same cell, and g's read after the call loads it fresh. Was 2,42 —
-- the face-local snapshot sat unowned between event boundaries.
local x = 2
local function F() x = x + 40 end
local function G() F() return x end
local g = G
print(g())
print(x)
