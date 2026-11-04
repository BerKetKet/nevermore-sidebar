# Phase L3: Music Library Migration Dry-Run Planning & Verification

**Date:** 2026-11-05  
**Tool:** `music-import` (installed to `/home/bgg/.local/bin/music-import`, tracked at `scripts/music-import`)  
**Source Directory:** `/home/bgg/Music/MusicBee`  
**Manifest File:** `docs/music_migration/manifest.tsv` (1,476 entries)  
**Undo Script:** `docs/music_migration/undo_migration.sh`  

---

## 1. Executive Summary

A complete, non-destructive, read-only dry run of the migration from the legacy MusicBee directory structure to the unified Linux/MPD ecosystem was executed.
Every one of the **1,476 files** (8.70 GB / 9,337,986,958 bytes) in the legacy repository was analyzed, verified for ID3/FLAC/MP4 tags, SHA-256 hashed, and mapped to its target location with 100% path safety.

### Category Breakdown

| Category | File Count | Destination Root | Description |
|:---|:---:|:---|:---|
| **Music Tracks** | 679 | `~/Music/` | Cleanly tagged audio tracks organized by `<Artist>/<Year> - <Album>/<Track>` |
| **Music Cover Art** | 142 | `~/Music/` | Primary album covers normalized to `cover.jpg` / `cover.png` in album folders |
| **Music Sidecars** | 73 | `~/Music/` | Synchronized `.lrc` and `.cue` files aligned with new track stems |
| **Audiobooks** | 115 | `~/Audio/Audiobooks/` | Spoken-word audiobooks (e.g. Arthur Golden, Steven Erikson) + cover |
| **Podcasts** | 16 | `~/Audio/Podcasts/` | Podcast subscription audio episodes and folders |
| **Archive: Skins** | 180 | `~/Archive/MusicBee-legacy/Skins/` | MusicBee XML skin files and theme assets |
| **Archive: Artist Pictures** | 79 | `~/Archive/MusicBee-legacy/Artist Pictures/` | Cached artist thumbnails and pictures |
| **Archive: Thumbnails** | 155 | `~/Archive/MusicBee-legacy/Thumbnails/` | WMP/MusicBee cache images (`AlbumArtSmall.jpg`, extra thumbnails) |
| **Archive: Database** | 9 | `~/Archive/MusicBee-legacy/Database/` | MusicBee library index, backup databases, settings (.mbl, .bak, .ini) |
| **Archive: Lyrics** | 6 | `~/Archive/MusicBee-legacy/lyrics_original/` | Original `#lyrics` folder contents preserved verbatim |
| **Archive: Playlists** | 3 | `~/Archive/MusicBee-legacy/Playlists/` | Proprietary MusicBee playlists (`.mbp`, `.xautopf`) |
| **Archive: Miscellaneous** | 15 | `~/Archive/MusicBee-legacy/Miscellaneous/` | Miscellaneous non-audio legacy files |
| **Quarantine: Duplicates** | 2 | `~/Music-Inbox/duplicates/` | Exact byte duplicates (identical SHA-256) |
| **Quarantine: Conflicts** | 2 | `~/Music-Inbox/conflicts/` | Duplicate filename with differing audio encodes (differing SHA-256) |
| **TOTAL** | **1,476** | | **100.0% accounting across legacy collection** |

---

## 2. Quarantined Files Detail

Zero data loss is guaranteed: conflicting and duplicate files are quarantined into `~/Music-Inbox` with unique stems and preserved tags:

1. **Exact Byte Duplicates (Identical SHA-256):**
   - Source: `Music/Dethklok/Metalocalypse  Dethklok The Dethalbum/14 Face Fisted_1.mp3`  
     SHA-256: `443f3b98c1f959e0b437c9699185036571c2c84611b641b84e6b13c44214968b`  
     Target: `~/Music-Inbox/duplicates/14 - Face Fisted.mp3`  
     *(Primary retained at `~/Music/Dethklok/2007 - The Dethalbum/14 - Face Fisted.mp3`)*
   - Source: `Music/Tool/Aenima/15 Third Eye_1.mp3`  
     SHA-256: `5481d595c0181926441783a2a3bad345717e83873122a86a861e8f8843ff4f46`  
     Target: `~/Music-Inbox/duplicates/15 - Third Eye.mp3`  
     *(Primary retained at `~/Music/Tool/1996 - Aenima/15 - Third Eye.mp3`)*

2. **File Collisions with Different Encodes (Differing SHA-256):**
   - Source: `Music/Dethklok/Metalocalypse  Dethklok The Dehtalbum/11 Birthday Dethday_1.mp3`  
     SHA-256: `a204ded6238a39470897bd87475fa26638cb01ad54166919834d2c644e46e9df`  
     Target: `~/Music-Inbox/conflicts/11 - Birthday Dethday.mp3`
   - Source: `Music/Tool/10,000 Days/01 Vicarious_1.mp3`  
     SHA-256: `4d99ce19c50dd1abf79774631c7aa9b10646ec6e84f6cdf93529df963979106a`  
     Target: `~/Music-Inbox/conflicts/01 - Vicarious.mp3`

---

## 3. Sample Mappings (20 Diverse Real-World Cases)

```tsv
# SOURCE_PATH	TARGET_PATH	CATEGORY
/home/bgg/Music/MusicBee/Music/AC DC/Blow Up Your Video/1-06 Nick of Time.mp3	/home/bgg/Music/AC_DC/2003 - Blow Up Your Video/06 - Nick of Time.mp3	Music
/home/bgg/Music/MusicBee/Music/AC DC/Blow Up Your Video/1-06 Nick of Time.lrc	/home/bgg/Music/AC_DC/2003 - Blow Up Your Video/06 - Nick of Time.lrc	Music/Sidecars
/home/bgg/Music/MusicBee/Music/AC DC/Blow Up Your Video/Folder.jpg	/home/bgg/Music/AC_DC/2003 - Blow Up Your Video/cover.jpg	Music/CoverArt
/home/bgg/Music/MusicBee/Music/Arthur Golden/Memoirs of A Geisha/004 - Chapter 2.mp3	/home/bgg/Audio/Audiobooks/Arthur Golden/Memoirs of A Geisha/04 - 004 - Chapter 2.mp3	Audio/Audiobooks
/home/bgg/Music/MusicBee/Music/BABYMETAL/BABYMETAL/1-01 BABYMETAL DEATH.mp3	/home/bgg/Music/BABYMETAL/2015 - BABYMETAL/01 - BABYMETAL DEATH.mp3	Music
/home/bgg/Music/MusicBee/Music/Billie Holiday/The Complete Decca Recordings (1944.10-1950.04)/2-20 Crazy He Calls Me (Single Version).flac	/home/bgg/Music/Billie Holiday/1991 - The Complete Decca Recordings (1944.10-1950.04)/2-20 - Crazy He Calls Me (Single Version).flac	Music
/home/bgg/Music/MusicBee/Music/Blue ?yster Cult/Extraterrestrial Live/1-06 Burnin' for You.mp3	/home/bgg/Music/Blue Öyster Cult/1982 - Extraterrestrial Live/06 - Burnin' for You.mp3	Music
/home/bgg/Music/MusicBee/Music/Dethklok/Dethalbum II/01 Bloodlines.mp3	/home/bgg/Music/Dethklok/2009 - Dethalbum II/01 - Bloodlines.mp3	Music
/home/bgg/Music/MusicBee/Music/Dethklok/Metalocalypse  Dethklok The Dethalbum/14 Face Fisted_1.mp3	/home/bgg/Music-Inbox/duplicates/14 - Face Fisted.mp3	Quarantine/duplicates
/home/bgg/Music/MusicBee/Music/Flight Of The Conchords/Flight of the Conchords/1-01 Foux Du Fafa.m4a	/home/bgg/Music/Flight Of The Conchords/2008 - Flight Of The Conchords/01 - Foux Du Fafa.m4a	Music
/home/bgg/Music/MusicBee/Music/Gigi/Gigi/01 Gud Fella.mp3	/home/bgg/Music/Gigi/2001 - Gigi/01 - Gud Fella.mp3	Music
/home/bgg/Music/MusicBee/Music/MAXIMUM THE HORMONE/A.S.A. Crew/01 See The Sea.mp3	/home/bgg/Music/MAXIMUM THE HORMONE/1999 - A.S.A. Crew/01 - See The Sea.mp3	Music
/home/bgg/Music/MusicBee/Music/Performed by Ralph Lister/Gardens of the Moon by Steven Erikson/01 B00 - Prologue.mp3	/home/bgg/Audio/Audiobooks/Performed by Ralph Lister/Gardens of the Moon by Steven Erikson/01 - B00 - Prologue.mp3	Audio/Audiobooks
/home/bgg/Music/MusicBee/Music/R??ni Wykonawcy; Various Artists/Kerrang! Metal Klassix/2-04 Hangar 18.mp3	/home/bgg/Music/Various Artists/1994 - Kerrang! Metal Klassix/2-04 - Hangar 18.mp3	Music
/home/bgg/Music/MusicBee/Music/Tool/10,000 Days/01 Vicarious_1.mp3	/home/bgg/Music-Inbox/conflicts/01 - Vicarious.mp3	Quarantine/conflicts
/home/bgg/Music/MusicBee/Podcasts/Brooke & Jeffrey Phone Taps/Phone Tap - Brooke (DJ Grilled Cheese).mp3	/home/bgg/Audio/Podcasts/Brooke & Jeffrey Phone Taps/Phone Tap - Brooke (DJ Grilled Cheese).mp3	Audio/Podcasts
/home/bgg/Music/MusicBee/Podcasts/The Snow Plow Show 2019 Arhive/TSPS 553 ? Last Will and Testament.mp3	/home/bgg/Audio/Podcasts/The Snow Plow Show 2019 Arhive/TSPS 553 _ Last Will and Testament.mp3	Audio/Podcasts
/home/bgg/Music/MusicBee/MusicBeeLibrary.bak	/home/bgg/Archive/MusicBee-legacy/Database/MusicBeeLibrary.bak	Archive/Database
/home/bgg/Music/MusicBee/Artist Pictures/Thumb/????? ? ????.jpg	/home/bgg/Archive/MusicBee-legacy/Artist Pictures/Thumb/????? ? ????.jpg	Archive/ArtistPictures
/home/bgg/Music/MusicBee/Skins/README.txt	/home/bgg/Archive/MusicBee-legacy/Skins/README.txt	Archive/Skins
```

---

## 4. Safety Verification

1. **Atomic Filesystem Boundary Check:**
   - Source (`/home/bgg/Music/MusicBee`) and target roots (`/home/bgg/Music`, `/home/bgg/Audio`, `/home/bgg/Archive`, `/home/bgg/Music-Inbox`) reside on Device `53` (`btrfs` subvolume `/@home`).
   - Every file operation is a zero-copy atomic inode `rename(2)`.
2. **Reflink Backup Snapshot:**
   - Verified active at `/home/bgg/Archive/MusicBee-reflink-backup/` created during Step L0.
3. **Undo Guarantee:**
   - `undo_migration.sh` provides 1-command complete reversal reading `manifest.tsv` in reverse order.
