---
created: 2024-12-30T18:11:41.696Z
updated: 2025-01-01T18:49:10.718Z
assigned: ""
progress: 0
tags:
  - App
started: 2024-12-31T21:36:27.210Z
completed: 2025-01-01T18:49:10.719Z
---

# Migrate spell query on recompile to a background task

After recompile, start fetching relevant spells and other database entries that are needed but shouldn't prevent the character from being "loaded". This should be moved out of recompile so it isn't blocking.

These requests should be each sent on their own async-spawn, and when the data is fetched and parsed, it gets inserted into a resolved-object-queue. On insert-when-empty, a 1-second timer is started (or some interval). When that timer expires, all inserted objects are moved to the character(handle)'s data (for spells, stored in spellcasting derived data). That way, data is laoded behind the scenes and propagated to the loaded character in batches.

There should be a mechanism that clears the "requested" set of entry ids on recompile, and any loaded objects which are not requested in a given recompile should be discarded.
