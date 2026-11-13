-- ARGS: 1
-- EXPECT: 0
-- EXPECT: 0
-- EXPECT: true
-- EXPECT: true
-- EXPECT: false
-- EXPECT: true
-- EXPECT: 0
-- EXPECT: !
-- EXPECT: 42
-- EXPECT: 42
-- EXPECT: bin
-- EXPECT: ada

-- The one-empty law: the literal "" and a missing cell's zero lane are
-- one value — the null handle — so len, ==, ~=, concat, and keying all
-- agree on it, and t[absent] and t[""] are the same slot.
local it = { hp = 10 }
local st = { who = "ada" }
local e = ""
print(#e)
local m = st.missing
print(#m)
print(m == "")
print("" == m)
print(m ~= "")
print(e == st.also_gone)
print(it.mana)
print(m .. "!")
local t = {}
t[m] = 42
print(t[m])
print(t[""])
t[""] = "bin"
print(t[m])
print(st.who)
return arg
