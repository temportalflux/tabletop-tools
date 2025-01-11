---
created: 2023-09-20T13:54:26.325Z
updated: 2025-01-11T14:06:51.736Z
assigned: ""
progress: 0
tags:
  - App
  - Offline
---

# Standalone App

Add tauri backend app for launching as a native windows exe. The frontend largely wont change b/c tauri allows yew/wasm for frontend rendering. Known unknowns include:
- how to read local files instead of fetching from github
- "downloading" local files for actual use via the indexed-db from the local file system; seems like tauri would be "fine" with that, but its unclear if the database would need to be constructed from scratch every time, or if its state maintains between launches.
- downloading modules from the web and putting in the local file system
