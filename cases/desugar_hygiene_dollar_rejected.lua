-- EXPECT_BUILD_FAIL: Syntax Error: Lexer error
-- Desugar hygiene is lexical: identifiers are [a-zA-Z_][a-zA-Z0-9_]*,
-- so '$' cannot appear in a user name and the synthetic namespace
-- (for$limit, tuple$tN) is unreachable from source. The census may
-- fingerprint desugars by those name strings precisely because this
-- rejection holds (archived LOGS §2, rule 2).
local tuple$t0 = 1
print(tuple$t0)
