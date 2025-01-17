---
created: 2023-09-20T13:51:39.013Z
updated: 2025-01-17T13:31:21.314Z
assigned: ""
progress: 0
tags:
  - App
  - v1
started: 2025-01-01T00:00:00.000Z
---

# Character Changelog

Character sheets no longer use mutation lambdas. Instead, mutating a sheet requires the action to create a discrete mutation structure instance, which is saved to the character sheet. When a sheet is given a new mutation, it performs the mutation locally to update the character sheet. This effectively just introdues a middle-step to sheet changes, but will allow those mutations to be saved to a changelog, viewed, and compared against remote versions of the character for diffing & merging.

The app keeps an in-memory changelog of mutations added since last save. Git commits are saved, w/ the long description interpretted as a kdl document, where each root-node is a kdl-ized mutation.

Users can mutate their characters while the app is checking latest versions and syncing. If the current version of a character in the client is not the exact same as the version in storage, then the app does a diff of these mutations. If the local user has any mutations exactly contained within the remote changes, the user is presented with a diff-merge warning. This popup informs the user of the mutations present in the remote/storage and how they differ from those in memory, noting the dates changes were saved and what the common root of the storage changes and local changes is. Users can choose to take one or the other, or replay their local changes on the remote (rebase).

Changelog is available in the sidebar, and specific changes can be selected to be "revertted" if the mutation supports it. This merely adds a new mutation that undoes the selected mutation.
