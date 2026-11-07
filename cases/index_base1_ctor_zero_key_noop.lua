-- FLAGS: --index-base=1
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 10	0
-- EXPECT: 2
-- EXPECT: 1	2	0
-- A literal [0] key under base 1 mints CtorKey::Const(-1) at the
-- parse choke point -- the first negative Const in the compiler's
-- history (at base 0 a bare Expr::Integer is never negative; negative
-- literals parse as UnaryOp(Neg) and ride CtorKey::Expr). Every Const
-- consumer is signed: the dedup set holds physical slots, the
-- threshold reads the shifted value, site_slots is keyed i64, and the
-- lowerer emits LoadInt -1 into the checked table_set, where the
-- runtime's negative-index no-op drops the entry -- the constructor
-- spelling of t[0] = 5 (zero_key_noop). The border counts only
-- in-domain entries, and a read below the base answers 0; positional
-- order is untouched ({10, [0] = 5} and {[0] = 5, 1, 2} keep 1 and 2
-- cells of payload).
local t = { [0] = 5 }
print(#t)
print(t[0])
local u = { 10, [0] = 5 }
print(#u)
print(u[1], u[0])
local v = { [0] = 5, 1, 2 }
print(#v)
print(v[1], v[2], v[0])
