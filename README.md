# Integro Tabletop
-----

integro (latin) - start from scratch, begin again, crecreate, renew

Integro Tabletop is a client-only web application used to facilitate stats traditionally tracked by pen-and-paper in tabletop games. Integro only currently supports the D&D 5e tabletop game (and only for character sheets) - and even that support is still in its fledgeling state.

## Why another character sheet app?

Why make a new character sheet app, especially for Dungeons & Dragons? There are tons of favorites out there; D&DBeyond, Roll20, Foundry, Demiplane, WorldAnvil, other products, and numerous indie creations.

Before D&DBeyond became well developed, used, & known, I tried a number of indie creations and Roll20 and none of them really fit my preferred playstle. At the time I primarily played D&D5e. When I found D&DBeyond, I used it for a number of years and was quite content, but the more I played with it, the more I longed for features that were so low priority they'd never get developed. Then consolidation of games and tech returned in 2022(ish), and I was ready to tinker with a new side project. So I decided to dig into the mechanical complexity of D&D to write my own character sheet, knowing there were a few features I especially wanted to make the pillars of the experience.

* Currency Exchange

I've never understood why D&D apps, which support the 5-6 kinds of currency found in the game, don't let me just automatically convert one to another. So one of the first features I implemented once I had sheets working was allowing players to opt-into currency exchange. Let me spend silver and coppper if all I have is platinum and gold. Let me exchange my 283 silver and 1429 copper into gold. Its just base-10 math exchanges.

* Stats and Effects always have a source

My biggest struggle with all current character sheets has always been that its difficult to track down where stat changes come from. Stat changes in this context are any changes to a number or roll; character ability scores like strength or intelligence, weapon attack and damage rolls, skill check bonuses, damage resistances, saving throw advantages, spells that I have preparred. Effectively anything that gets changed from the default empty state of a character, usually by features, items, spells, conditions. I wanted to know exactly why I had advantage on constitution saving throws so I as a DM or a player could always open that information and see "oh this comes from a feature named X on my character, which was added by this class/species/ancestry/condition".

* Spell Containers

As someone who'd often play a Wizard, I wanted to be able to know exactly what spells are in my spellbook (or to have multiple separate spellbooks). To use magic items like spell-scrolls and spell-gems and items which in theory contain spells. None of the existing character sheets supported this functionality. Many now have item containers (backpacks, chests, etc), but no way to put spells of certain level+DC+Attack stats in an item which supports it.

* Homebrew Customizations

Homebrew on D&DBeyond (when I still used it) was less than ideal. It was a lot of trial and error to try to find a way to create an item or class or feat which gave the character a specific change that I knew other items could do. It was a very opaque and error prone editor. So I knew I wanted to make a system where it was easy to understand the data of an item if you looked at it in plain-text AND which could be easily wrapped with a UI which was able to be quickly understood and attached to any character-related data (class, ancestry, feat, condition, item, spell, etc).

This also extends to adding custom feats and conditions to characters mid-session. I want it to be easy to add a previously undefined condition or feature to a character and be able to very rapidly identify how that feature affects the character sheet (e.g. grants a single-use-per-short-rest spell).

* User-Owned Data (& Offline)

We've gotten to a point of digitial feudalism where I'm just really tired of products and companies owning my data. I dont want to have to connect to a webapp's backend server to access my characters and play my silly little tabletop games. My data should be stored on my devices. So a goal of this project is to grant access to rulesets and content but to have them stored on the device to be used. There is no backend server. All of the character is stored in the device's storage. Eventually I'd like Integro to support a fully-offline desktop app format, where the content is stored to a specific folder on the user's computer.

I also acknowledge the need and desire for my content to be synced across devices though, and doing that without some shared backend is nigh impossible. So Integro walks the gap, both storing the app's data on the local device, and syncing each user's homebrew, character, and app settings to their own self-owned github repository(s). One day perhaps this app will support multiple storage backends.

* Multi-System

There continues to be numerous interesting game systems out there to try, and I'd like to play them all in a multi-device webapp where I have full control over my own data. So the infastructure for Integro has been built with keen intent towards supporting multiple game systems. Transcribing those game systems into content though is time-consuming, so it only supports D&D5e (2014, and slowly 2024) at the moment.

* Third-Party creations

With the reality of many tabletop apps being walled gardens, and stored on that product's backend, I knew I also wanted to be able to play with third-party content very easily. It should be trivial for creators to control their own integro module for any game system (and their content to support multiple game systems), and to grant access to players at their discretion (e.g. consumers getting access to Roll20 modules when you purchase a book or pdf).

This will be far from perfect however, and will likely necessitate running on an honor system due to conflicts with the pillar of users owning their own data.

* Character Changelog

Another piece of UX I realized I desired out of pre-existing character sheets was a way to view the history of changes to my character; when and what items I added/removed, how much health I lost to what creature (e.g. during combat), and all number of other changes.

Also from a serverless persistent-data tech perspective, its very useful to know how many changes have been made since the last time a character has been saved. Especially if you know what those changes were. So having a changelog enables the app to more robustly support auto-saving.

* Parcels / Mailbox

I find in many of my tabletop game sessions, either party members will want to swap items, or the DM will grant items and loot to players. And this is usually by word of mouth, which is easily lost track of in the moment as players are trying to find items in the app and add the correct amount of currency to their character. Thus the desire for a mailbox and parcels came to be. A place where players could check their character's mailbox for parcels from other players and their GM, parcels which may contain secret text notes, items, and currency. Players would be able to send items and currency in their inventory to another player, which can be accepted or returned, all without the worry of losing track of who has what item when its talked about above the table.

* Party View

One day I'd be rad to support a party view; a way for game masters to see all of the high-level stats of the party members and be able to open their character sheet in a read-only mode. This would likely also go hand in hand with mailboxes, and would support a way for GMs to prep mail/parcels to send to players on the fly.

* Spell Transcription

For Wizards and some Warlocks in D&D5e, spells must be transcribed from scrolls or other spellbooks in order to be used in one's own spellbook. I wanted to be able to do this mechanically in the app rather than in above-the-table talk. Especially with the existance of spell containers.

* Spell Components

Having the necessary material components for spells is nigh impossible to track in pre-existing D&D sheet apps. Not everyone even uses these rules. I'd be rad to be able to allow players to opt-into tracking this information, so they know what spells they can and cannot cast based on what components they have.

* Tokens, Inspiration, and Concentration

GM-granted Inspiration is often supported as a basic flag on the character sheet, which players can use to roll advantage on some check/test - but things like Bardic inspiration are not supported. There's no way to track what my character is currently concentrating on (e.g. spells). And all of those are forms of tokens, which can be used to adapt rules from other systems (e.g. the flashback mechanic in heist styled gameplay).

* Combat Tracker

Who knows, maybe someday down the road I'll aspire to expand the app to support non-player creatures with a combat & hit-point tracker that shows live data updates as creatures and players take damage or acquire conditions. Thats way down the road though.
