---
created: 2024-11-08T21:31:07.037Z
updated: 2024-11-09T15:16:00.021Z
assigned: ""
progress: 0
tags:
  - App
started: 2024-11-08T21:31:55.227Z
completed: 2024-11-09T15:16:00.021Z
---

# Add a spell object cache for character sheets

Spellcasting::fetch_spell_objects gets executed on every recompile and always fetches spells from the database. It would significantly improve character recompile async time to have a cache alongside the database object, which allows objects to be held onto for some period of time before discarded. This way fetching spells doesnt have to query the database and parse all relevant objects every time.
The ObjectCacheProvider wrapper would be a good place for this.
