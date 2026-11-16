-- EXPECT: 1
-- EXPECT: 2
-- XFAIL: judge_divergence_audit/fn-rebind — a direct name call must
-- resolve at its call site's binding point; today every call of the
-- name rides the walk's final collected literal, so the first print
-- answers 2 before g() ever ran. The judge prints 1 then 2. Flip
-- when resolve_fn_binding resolves through the call point's binding.
local f = function() return 1 end
local g = function() f = function() return 2 end end
print(f())
g()
print(f())
