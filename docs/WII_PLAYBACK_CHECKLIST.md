# Wii playback checklist

Status: pending physical hardware validation. Desktop tests do not certify Wii
playback. Keep original videos while checking the encoded versions.

Use your normal Wii media player and Wii/PVM display settings, and keep those
settings fixed while comparing samples.

| Sample | Check |
| --- | --- |
| 4:3 and widescreen anime | Correct geometry and padding; no unintended stretching |
| Action scene and slow camera pan | Smooth motion and acceptable compression |
| SRT and styled ASS | Readable text, correct timing, outlines, and placement |
| Japanese names and punctuation | Correct characters and expected font rendering |
| Dialogue, music, and quiet passages | Correct channel balance, no clipping, and A/V sync |
| Longer episode | Stable playback and sync near the end |
| Seeking backward/forward | Playback resumes with synchronized audio |
| Available GPU and CPU outputs | Both play correctly; compare quality as well as speed |
| FAT32 copy | Each file fits below 4 GiB; copying and playback succeed |

Also check pause/resume during a desktop encode, close-to-tray operation, and
recovery after an interrupted queue. Record the player version, Wii display
mode, sample name, and result for any problem. Test a full episode before a
large conversion batch or public release.
