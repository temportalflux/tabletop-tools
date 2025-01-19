---
created: 2023-09-20T13:46:03.506Z
updated: 2025-01-18T19:28:34.688Z
assigned: ""
progress: 0
tags:
  - Optimization
started: 2025-01-18T19:28:34.688Z
---

# Expanded Object Cache

Additional Object Cache should save its bundles to the character kdl (and read them) so offline character sheets can be supported. Ideally we'd track what bundles are actually being used by the charcter so we don't reserialize bundles that aren't actively in use.
