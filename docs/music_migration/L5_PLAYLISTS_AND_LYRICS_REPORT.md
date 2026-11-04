# Phase L5: Playlists & Lyrics Conversion to MPD Standards

**Date:** 2026-11-05  
**Ecosystem:** Linux MPD / Nevermore Sidebar Music Plugin / Euphonica  
**Status:** COMPLETE & VERIFIED  

---

## 1. Lyrics Standardization

In the legacy library, lyrics were divided between album folder sidecars and a detached `#lyrics` folder. Both have been fully reconciled:

### A. Detached `#lyrics` Files (6 files)
All 6 files from `~/Archive/MusicBee-legacy/lyrics_original/` were verified for standard LRC timestamp syntax and copied directly next to their matching migrated audio tracks with `.lrc` extension, leaving the archived originals completely untouched:

1. `03 Awaken.lrc`  
   -> `~/Music/Dethklok/2007 - Metalocalypse - Dethklok The Dethalbum/03 - Awaken.lrc`
2. `03 Burn the Earth.lrc`  
   -> `~/Music/Dethklok/2009 - Dethalbum II/03 - Burn the Earth.lrc`
3. `03 Hiphopopotamus Vs. Rhymenoceros.lrc`  
   -> `~/Music/Flight Of The Conchords/2008 - Flight Of The Conchords/03 - Hiphopopotamus Vs. Rhymenoceros.lrc`
4. `04 Bloodrocuted.lrc`  
   -> `~/Music/Dethklok/2007 - Metalocalypse - Dethklok The Dethalbum/04 - Bloodrocuted.lrc`
5. `1-01 Foux Du Fafa.lrc`  
   -> `~/Music/Flight Of The Conchords/2008 - Flight Of The Conchords/01 - Foux Du Fafa.lrc`
6. `1-02 Inner City Pressure.lrc`  
   -> `~/Music/Flight Of The Conchords/2008 - Flight Of The Conchords/02 - Inner City Pressure.lrc`

### B. In-Album Sidecars (73 files)
The 73 `.lrc` sidecars already embedded within album directories were automatically renamed to match the exact standardized track stem during Step L4 (e.g. `1-06 Nick of Time.lrc` -> `06 - Nick of Time.lrc`). All 79 total lyrics files in `~/Music/` are now positioned adjacent to their corresponding audio files and discoverable by MPD lyrics engines.

---

## 2. Playlists Audit & Euphonica Transition

The legacy MusicBee library contained no static `.m3u` or `.m3u8` playlist files; all playlist data was stored in proprietary MusicBee formats:

### Legacy Playlists Cataloged

| Playlist File | Format | Purpose | Action Taken |
|:---|:---|:---|:---|
| `NowPlaying.mbp` | Windows binary stream | Active playback queue pointer (`C:\Users\berke\...`) | Preserved in `~/Archive/MusicBee-legacy/Database/` and `Playlists/` |
| `Recently Added.xautopf` | MusicBee XML Smart Playlist | Dynamic query filter: `AlbumDateAdded InTheLast 6m` | Preserved in `~/Archive/MusicBee-legacy/Playlists/` |
| `Top 25 Most Played.xautopf` | MusicBee XML Smart Playlist | Dynamic query filter: Top 25 items by playcount | Preserved in `~/Archive/MusicBee-legacy/Playlists/` |

### Euphonica / MPD Recreation Recommendations

MPD stores static playlists as plain text relative paths in `~/.config/mpd/playlists/`. Dynamic playlists in modern MPD clients (such as Euphonica, ncmpcpp, and Cantata) are created as follows:
- **Recently Added:** In Euphonica or via `mpc search "(date >= '2026')"` or sorting the library view by file modification time (`mtime`).
- **Top Most Played:** Supported in Euphonica via its local SQLite playcount tracker or via `mpd-statistic` daemons.
- **Queue Persistence:** Handled automatically by MPD's `state_file "~/.local/state/mpd/state"` without requiring proprietary `.mbp` files.
