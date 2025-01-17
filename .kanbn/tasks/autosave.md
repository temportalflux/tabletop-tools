---
created: 2023-09-20T13:43:40.933Z
updated: 2025-01-17T13:27:52.888Z
assigned: ""
progress: 0
tags:
  - Persistence
  - v1
---

# Autosave

- Leaving editor always saves
- All changes generate a changelog message
- Adjacent changelog messages with the same type will auto-combine (e.g. hit point increment & decrement, excluding bulk changes)
- If there are no additional changes within 60 seconds, auto save
- Display save timer and a manual save button in header of display sheet under the rest & builder buttons
- Button to open changelog and view both committed and uncommitted changes
