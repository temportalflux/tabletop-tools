---
created: 2023-09-29T22:25:16.360Z
updated: 2024-11-16T15:36:50.260Z
assigned: ""
progress: 0
tags:
  - Capability
completed: 2024-11-16T15:36:50.260Z
---

# Add "restore_uses" mutator

Mutator which restores some number of uses to a resource when a rest of a particular type is taken. (e.g. Sorcerer's "Sorcerous Restoration")
```
mutator "restore_uses" reset_on="Short" {
	amount 4
	resource "Sorcerer/level02/Font of Magic"
}
```
