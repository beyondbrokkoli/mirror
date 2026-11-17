-- EXPECT: 2
-- The p9 reversal retired this program's rejection: a closure-side
-- rebind of a captured table is the judge's upvalue semantics, and
-- since the box-is-binding law cells every captured Tbl at
-- declaration, the rebind routes through THE cell — Step's Row store
-- lands the new table in it and reclaims the displaced one (single
-- ownership), and Peek's expansion reads the same cell. Formerly
-- pinned negative ('u' is a captured table — it cannot be rebound
-- inside its closure), 2026-11-15; flipped with the boxing
-- (leak_contract/p9). Peek exists to prove the rebind reaches every
-- holder, not just the rebinding surface.
local function Make()
    local u = { [0] = 1 }
    local function Step() u = { [0] = 2 } end
    local function Peek() return u[0] end
    return { Step = Step, Peek = Peek }
end
local s = Make()
s.Step()
print(s.Peek())
