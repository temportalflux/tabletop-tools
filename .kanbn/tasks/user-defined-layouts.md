---
created: 2025-01-11T14:06:23.796Z
updated: 2025-01-11T14:06:50.389Z
assigned: ""
progress: 0
tags:
  - App
---

# User-Defined Layouts

This work would unify both joined and paged sheet displays, by having sheet layouts defined by data. The app has a set of permanent defaults which all users opt-in to by default.

A user can have multiple layout templates saved to their user data. A user can open the settings for a character to select the app or a layout template to use for their character. They can also customize the active layout on a per-character basis (and have a way to save that layout back to their user settings).

A character has its "default" layout and a per-device layout (if the latter is not set, the former is used) - which will cover the differences between mobile and PC.

A layout is a set of pages, each with a arrangement of property displays currated by the app. These displays are basically arranged in flex-boxes (would likely need support for horizontal and vertical layout elements with horizontal and vertical alignment options).
