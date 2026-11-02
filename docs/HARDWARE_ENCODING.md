# Hardware encoding

Before the first episode that needs encoding, the backend runs the bundled
`ffmpeg -encoders` without a console window. It parses exact video encoder names
and tries `h264_nvenc`, `h264_amf`, then `h264_qsv`. Each listed encoder must also
encode two synthetic SD frames successfully: compiled support alone does not
prove that a compatible GPU and driver are available. Probes are tracked like
other media children, accept cancellation, have an eight-second timeout, and
are reaped before the next process starts. An all-skipped batch does not run
encoder detection.

Advanced Mode can select Auto, NVIDIA NVENC, AMD AMF, Intel QSV, or CPU. A GPU
override restricts probing to that GPU, with CPU fallback if it is unavailable.
CPU skips GPU probing. These settings are saved with preferences and checkpoints.

| Encoder | Preset / rate control |
| --- | --- |
| NVIDIA NVENC | `p4`, VBR, CQ from the selected quality value |
| AMD AMF | `quality`, peak-constrained VBR |
| Intel QSV | `fast`, VBR, lookahead disabled |
| CPU libx264 | `superfast`, CRF, `-threads 0` |

Hardware bitrate targets map quality 20/23/26 to 2000/1500/1000 kbps. These are
encoder-specific settings and do not promise identical quality to x264 CRF.
Maximum mode uses automatic software threading. Balanced uses the existing
logical-cores-minus-two budget, with a minimum of one thread, and below-normal
FFmpeg priority on Windows. This is a thread-budget/priority choice, not an
exclusive reservation of physical CPU cores.

Decoding, scaling, padding, and subtitle burn-in remain in software. No GPU
decode or hardware-frame filter is enabled. NVENC accepts the resulting yuv420p
frames directly; QSV/AMF may negotiate NV12 input surfaces internally. NV12 is
also 8-bit 4:2:0, and the encoded H.264 stream remains 4:2:0 rather than 4:4:4.
The encoder handles the final upload after software filters have finished.

Every path uses the selected SD raster and display aspect, at most 30 fps
(25 fps for 848x480), Main profile, Level 3.1, no B-frames, 3.2 Mbps VBV maximum rate with a 6 Mb
buffer, AAC-LC stereo at 160 kbps/48 kHz, and MP4 fast-start metadata.
Wii player compatibility and speed still need testing on the user's hardware;
CPU subtitle rendering can remain the bottleneck.

The selected encoder is reused within the batch and redetected for a later or
recovered batch. Existing outputs are reused according to their provenance records; unrecorded legacy files are preserved with a warning. If hardware FFmpeg
exits unsuccessfully or its result fails output validation, its process is reaped and its owned `.part` file is removed
before one CPU retry. The rest of that queue then uses CPU encoding. Cancellation,
checkpoint/commit errors, existing final outputs, and unsuccessful partial-file
cleanup do not trigger that retry. Successful outputs retain the existing
atomic finalization and checkpoint flow. Windows pause/resume still controls the
same managed FFmpeg process, including hardware-encoding worker threads.

Encoder choices and probe/driver failures go to the local diagnostic log.
They do not become spurious per-episode UI warnings.

See [aspect ratio and resolution](ASPECT_RATIO_AND_RESOLUTION.md) for anamorphic SAR, custom sizing, and the conservative frame-rate budget.
