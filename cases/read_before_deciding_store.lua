-- EXPECT: nil
-- XFAIL: judge_divergence_audit/absent-nil — the scalar read of a key
-- never written answers 0 (absent ≡ zero); the judge answers nil.
-- Flip when absent scalar reads answer nil.
local t = {}
local x = t[0]
if 1 < 2 then
  t[0] = 5
end
print(x)
t = nil
