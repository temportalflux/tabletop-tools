---
created: 2024-12-31T21:36:06.944Z
updated: 2024-12-31T21:36:11.977Z
assigned: ""
progress: 0
tags:
  - App
---

# Embed item and condition indirection in characters

Instead of characters being able to have Indirect source ids on items and conditions - any objects added to persistent data should have their indirection resolved on insert (or before).

Add asserts/logging when characters contain objects (items/conditions) with indirection.

This will contribute to characters always being able to be loaded regardless of what content is installed.
