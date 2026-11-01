# Encoding benchmark

Synthetic 30-second 1280x720/24 fps test pattern plus sine audio, encoded through the production 640x480 pipeline. Rust development/test harness; desktop UI and batch checkpoint hashing overhead are excluded. One sample per case; this is not a real-anime quality assessment or Wii playback test. Wall time includes probing and output verification. Variations from driver warm-up, load, and storage are expected.

| Encoder | Subtitles | CPU mode | Wall s | Rate | User CPU s | Peak MiB | Output MiB |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| h264_nvenc | None | Maximum | 0.713 | 42.05x | 1.05 | 338.15 | 4.97 |
| h264_nvenc | SRT | Maximum | 0.802 | 37.40x | 1.69 | 343.03 | 5.03 |
| h264_nvenc | Styled ASS | Maximum | 1.901 | 15.78x | 2.52 | 427.16 | 6.34 |
| h264_nvenc | None | Balanced | 0.716 | 41.90x | 0.86 | 330.71 | 4.97 |
| h264_nvenc | SRT | Balanced | 0.750 | 40.00x | 1.22 | 335.62 | 5.03 |
| h264_nvenc | Styled ASS | Balanced | 1.907 | 15.73x | 2.58 | 418.32 | 6.34 |
| libx264 | None | Maximum | 0.655 | 45.83x | 2.86 | 162.52 | 4.62 |
| libx264 | SRT | Maximum | 0.691 | 43.45x | 2.30 | 167.18 | 4.67 |
| libx264 | Styled ASS | Maximum | 1.665 | 18.02x | 2.42 | 253.73 | 6.23 |
| libx264 | None | Balanced | 0.630 | 47.58x | 2.91 | 151.03 | 4.62 |
| libx264 | SRT | Balanced | 0.644 | 46.57x | 2.36 | 155.45 | 4.67 |
| libx264 | Styled ASS | Balanced | 1.721 | 17.43x | 2.45 | 242.20 | 6.24 |

Use repeat runs and your own episodes before choosing a default. Maximum remains the existing default; Balanced reserves CPU thread capacity for desktop use. No speedup is guaranteed for every source.
