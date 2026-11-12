-- M3 acceptance: the parent requires the child, the child's
-- top-level runs exactly once at the require site, and the returned
-- table's closure answers through the parent's binding.
-- FLAGS: --module-root=cases/modules
-- EXPECT: child: top-level runs
-- EXPECT: hello engine
local G = require("child.greeter")
print(G.hello("engine"))
