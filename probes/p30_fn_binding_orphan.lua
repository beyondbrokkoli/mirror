-- P30 (fortification): a fn entry rides its name's binding depth —
-- `assign_fn_binding` now scans the master `scopes` stack for the
-- binding's home depth and writes `fn_scopes` there, so a nested
-- assignment may not orphan the fn-ness in an inner map that dies with
-- its scope. The T2 scalar-node ghost's living sibling (same birth
-- shape: rider-stack rposition + innermost fallback, functions.rs).
--
-- RECEIPT (2026-11-14, pre-fix binary at d745f58 vs post-fix tree):
--   pre-fix  : line 4: Type Error: 'arg' is not an expansion — a call
--              through a value needs a closure — the fn-ness parked in
--              the Do's fn_scopes map (no fn entry existed for 'arg' at
--              any depth: the boundary root binding has none), the
--              fallback wrote it innermost, and exit_scope buried it.
--   post-fix : compiles; prints 7
--   judge    : luajit prints 7
--
-- The decl-assigned twin (local f = 5 -> do f = fn end -> f()) passed
-- even pre-fix — the decl writes the fn entry at the outer depth, so
-- the orphan needs a binding with no fn entry at any rider depth.
-- 'arg', the boundary root, is exactly that name. The fix's master
-- scan is what makes the writer trust the binding's home depth instead
-- of wherever the rider stack happens to hold a stale entry.

-- the orphan: fn-ness assigned inside a block over a binding that has
-- no fn entry — must survive the block's exit
do
  arg = function() return 7 end
end
print(arg())

-- the shadow twin keeps its inner face; the outer fn entry survives
-- the inner decl untouched
local g = function() return 1 end
do
  local g = 5
end
print(g())
