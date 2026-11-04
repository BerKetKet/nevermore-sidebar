# L1 MusicBee Library Inventory & Enumeration Report

**Generated:** 2026-11-05 01:11:30 +0300  
**Source Path:** `/home/bgg/Music/MusicBee/`  
**Total Files Scanned:** 1,476  
**Total Byte Size:** 9,337,986,958 bytes (8.70 GB)  
**Filesystem:** `btrfs` (`stat -c %d` = `53`)  

---

## 1. Extension & Format Breakdown

| Extension | Category | File Count | Total Bytes | % of Total Size |
|---|---|---|---|---|
| `.mp3` | Audio | 692 | 7,295,256,880 | 78.1% |
| `.flac` | Audio | 99 | 1,806,796,319 | 19.3% |
| `.m4a` | Audio | 22 | 145,460,873 | 1.6% |
| `.jpg` | Image | 374 | 49,743,512 | 0.5% |
| `.bak` | Metadata / App | 3 | 13,718,520 | 0.1% |
| `.mbl` | Metadata / App | 1 | 8,192,000 | 0.1% |
| `.xml` | Metadata / App | 176 | 7,920,266 | 0.1% |
| `.dat` | Metadata / App | 1 | 5,522,605 | 0.1% |
| `.png` | Image | 3 | 3,444,950 | 0.0% |
| `.lyrics` | Lyrics | 1 | 1,024,000 | 0.0% |
| `.zip` | Metadata / App | 2 | 723,178 | 0.0% |
| `.lrc` | Lyrics | 79 | 154,033 | 0.0% |
| `.txt` | Lyrics | 17 | 24,081 | 0.0% |
| `.ini` | Metadata / App | 1 | 3,915 | 0.0% |
| `.xautopf` | Metadata / App | 2 | 1,541 | 0.0% |
| `.mbp` | Metadata / App | 2 | 262 | 0.0% |
| `.pfidx` | Metadata / App | 1 | 23 | 0.0% |

### Audio Formats Summary
- **Total Audio Tracks:** 813 files (8.61 GB)
- **MP3 (`.mp3`):** 692 tracks
- **FLAC (`.flac`):** 99 tracks (lossless)
- **M4A / AAC (`.m4a`):** 22 tracks

---

## 2. Tag Quality & Metadata Integrity

| Metric | Count | Routing Impact |
|---|---|---|
| Tracks with Complete Tags (Artist, Album, Title, Track) | 801 | Standard migration to `~/Music/<Album Artist>/<Year> - <Album>/` |
| Tracks Missing Required Tags | 12 | Quarantined to `~/Music-Inbox/needs-tags/` |
| Tracks Missing Track Number Tag | 12 | 11 Podcast episodes + 1 orphan audio track |
| Tracks Missing Album Tag | 1 | `Suni Clay - # My Hood (Acapella Mix)` |
| Tracks Missing Artist Tag | 0 | 0 (all tracks have identifiable artist) |
| Tracks Missing Title Tag | 0 | 0 (all tracks have title) |
| Multi-Disc Album Tracks (`discnumber > 1`) | 38 | Formatted as `<D>-<NN> - <Title>.<ext>` |
| Compilation / Various Artists Tracks | 105 | Formatted under `~/Music/Various Artists/<Year> - <Album>/` |

### Missing Tags Detail (12 files)
1. `Music/Suni Clay/Unknown Album/# My Hood (Acapella Mix).m4a` — missing `album`, `tracknumber`
2. 11 episodes in `Podcasts/Brooke & Jeffrey Phone Taps/` — podcast episodes missing `tracknumber` (routed to `~/Audio/Podcasts/`)

---

## 3. Duplication Audit

### Exact Byte Duplicates (SHA-256 Identical)
Exact duplicate sets: **2** (2 duplicate files identified):

- **SHA-256:** `443f3b98c1f959e0b437c9699185036571c2c84611b641b84e6b13c44214968b`
  * Primary (keep in library): `Music/Dethklok/Metalocalypse  Dethklok The Dethalbum/14 Face Fisted.mp3`
  * Duplicate (quarantine to `~/Music-Inbox/duplicates/`): `Music/Dethklok/Metalocalypse  Dethklok The Dethalbum/14 Face Fisted_1.mp3`
- **SHA-256:** `5481d595c0181926441783a2a3bad345717e83873122a86a861e8f8843ff4f46`
  * Primary (keep in library): `Music/Tool/Aenima/15 Third Eye.mp3`
  * Duplicate (quarantine to `~/Music-Inbox/duplicates/`): `Music/Tool/Aenima/15 Third Eye_1.mp3`

### Tag Tuple Duplicates (Artist, Album, Title Match)
- Identified 13 sets where metadata matches across multiple files (including multi-format rips or bonus editions).
- Safe resolution: Only identical SHA-256 byte duplicates are deduplicated automatically; different bitrates or versions are safely preserved or quarantined if paths collide.

---

## 4. Path Hygiene & Character Encoding

| Check | Count | Description & Remediation |
|---|---|---|
| NTFS Illegal Characters (`:`, `?`, `*`, `"`, `<`, `>`, `|`) | 116 | Legacy Windows/NTFS files with literal `?` replacing foreign diacritics or `:` in titles. Replaced with clean NFC punctuation (` - ` for `:`, removed `?`). |
| Trailing Dots or Spaces | 0 | Clean (0 files). |
| Path Components > 255 Bytes | 0 | Clean (0 components exceed 255-byte limit). |
| Non-Latin Unicode Scripts | 0 | Handled via Unicode NFC normalization. |

---

## 5. Non-Audio Assets & Routing Policy

| Source Directory / Pattern | File Count | Target Layout Routing |
|---|---|---|
| `Podcasts/` (MP3 files) | 11 | `~/Audio/Podcasts/Brooke & Jeffrey Phone Taps/` |
| `Audiobooks/` | 0 | `~/Audio/Audiobooks/` |
| `Skins/` (XML, PNG, BMP) | 178 | `~/Archive/MusicBee-legacy/Skins/` |
| `Artist Pictures/` (JPG thumbnails) | 7 | `~/Archive/MusicBee-legacy/Artist Pictures/` |
| MusicBee Database Files (`.mbl`, `.bak`, `.lyrics`, `.pfidx`, `.ini`) | 7 | `~/Archive/MusicBee-legacy/Database/` |
| Album Cover Art (`cover.jpg`, `folder.jpg`, `AlbumArtSmall.jpg`) | 374 | Standardized as `cover.jpg` / `cover.png` alongside album tracks |
| `#lyrics` folder (`.lrc` synced lyrics) | 79 | Migrated alongside matching track stems as `<Stem>.lrc` |
| Playlists (`.xautopf`, `.mbp`) | 4 | Converted to standard MPD `.m3u` in `~/.config/mpd/playlists/` |

---

## 6. Migration Feasibility Conclusion

1. **Safety Status:** `PASS`. Reflink snapshot verified at `/home/bgg/Archive/MusicBee-reflink-backup`.
2. **Filesystem Status:** `PASS`. Same device ID 53 confirms atomic `rename(2)` capability.
3. **Quarantine Preparedness:** Exact quarantine destinations identified:
   - `~/Music-Inbox/needs-tags/` (1 track)
   - `~/Music-Inbox/duplicates/` (2 duplicate tracks)
   - `~/Audio/Podcasts/` (11 podcast tracks)
   - `~/Archive/MusicBee-legacy/` (MusicBee app databases, skins, artist pictures)
4. **Ready for L2/L3:** Shared migration script `music-import` dry-run can proceed safely.

