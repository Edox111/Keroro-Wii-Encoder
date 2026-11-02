# Aspect ratio and SD resolution

The production frontend uses TypeScript and CSS. Its selectors live in
`index.html`, with persistence, IPC, and recovery handling in `src/main.ts` and
`src/preferences.ts`. Existing preferences/checkpoints default to 640x480, 4:3.

Resolution chooses the **stored pixel dimensions**. Aspect ratio independently
chooses the **display shape**. The presets are:

| Raster | Typical display selection | Frame-rate cap |
| --- | --- | --- |
| 640x480 | 4:3 | 30 fps |
| 848x480 | 16:9 (near-square pixels) | 25 fps |
| 720x480 | 4:3 or 16:9 anamorphic | 30 fps |
| Custom | Auto, 4:3, or 16:9 | Calculated from dimensions, at most 30 fps |

Custom dimensions auto-round to even values (ties upward): width 2–854, height 2–480. These are encoding
limits, not certification of every size on physical Wii hardware.

**Auto (Keep Source)** preserves the input display aspect ratio using FFmpeg's
sample-aspect adjustment. For example, a 16:9 source encoded into a 640x480
storage raster remains 16:9 when played by a player honoring SAR/DAR metadata.
Explicit **4:3** or **16:9** fits the source into that display canvas, adding
letterboxing or pillarboxing as necessary instead of stretching the picture.
Optional auto-crop can remove detected black borders before fitting. There is no stretch override. See [crop detection](AUTO_CROP.md) and the [WiiMC profile](WIIMC_PROFILE.md).

For **720x480**, select 4:3 for SAR 8:9 or 16:9 for SAR 32:27. Do not force
`setsar=1` after building an anamorphic filter: it would change the display shape.
An 848x480 raster is only approximately 16:9 with square pixels; exact 16:9
uses SAR 160:159. The recommended 720x480 widescreen preset uses SAR 32:27.
Legacy 854x480 preset values deserialize to 848x480; 854 is no longer exposed as a preset.

## Encoding implementation

`VideoSettings::filter()` in `src-tauri/crates/encoder-core/src/video.rs` builds
the filter with validated dimensions and enum-controlled aspect ratios. For
explicit DAR, it computes target SAR as `DAR * height / width` and fits source
`dar` (which includes source SAR) into the stored raster. Padding, DAR metadata,
and `yuv420p` formatting are followed by the existing subtitle burn-in filter.
All operations remain one `-vf` argument; no shell execution is involved.

All encoders now signal Main Level 3.1. The application deliberately retains a
conservative 40,500 macroblocks/s workload budget and a 30 fps maximum, rather
than using the larger workload permitted by that level. Thus 848x480 is capped
at 25 fps, while 640x480 and 720x480 allow 30 fps.
GPU and CPU encoders use the same cap and the existing audio/bitrate/profile
constraints. The output validator checks the selected size, DAR where explicit,
and frame-rate budget before finalization.

Format settings are saved in preferences and checkpoints, and included in output
reuse records. A changed resolution/aspect request preserves an existing output
and reports a conflict rather than treating it as a matching completed file.

## Equivalent React / Tailwind component

This reference component is not required by the existing frontend. Supply a
`value` object matching `src/video.ts` and validate it before submission.

```jsx
export function VideoControls({ value, onChange, disabled = false }) {
  const patch = (field, next) => onChange({ ...value, [field]: next });
  const field = "rounded-lg border border-white/20 bg-slate-950/70 p-2 text-white";

  return (
    <fieldset disabled={disabled} className="grid grid-cols-2 gap-3">
      <label className="grid gap-1 text-sm text-slate-200">
        Aspect ratio
        <select className={field} value={value.aspectRatio}
          onChange={e => patch("aspectRatio", e.target.value)}>
          <option value="auto">Auto (Keep Source)</option>
          <option value="4:3">4:3 (Standard CRT)</option>
          <option value="16:9">16:9 (Widescreen)</option>
        </select>
      </label>
      <label className="grid gap-1 text-sm text-slate-200">
        Resolution
        <select className={field} value={value.resolutionPreset}
          onChange={e => patch("resolutionPreset", e.target.value)}>
          <option value="640x480">640x480 (Standard 4:3)</option>
          <option value="848x480">848x480 (Mod-16, Near-Square Pixels)</option>
          <option value="720x480">720x480 (Anamorphic NTSC DVD)</option>
          <option value="custom">Custom</option>
        </select>
      </label>
      {value.resolutionPreset === "custom" && [
        ["customWidth", "Width", 854], ["customHeight", "Height", 480],
      ].map(([name, label, max]) => (
        <label key={name} className="grid gap-1 text-sm text-slate-200">
          {label}
          <input className={field} type="number" min={2} max={max} step={2}
            required value={value[name] || ""}
            onChange={e => patch(name, e.target.valueAsNumber || 0)} />
        </label>
      ))}
    </fieldset>
  );
}
```

References: [Chromium's H.264 level-limit implementation](https://chromium.googlesource.com/chromium/src/+/lkgr/media/parsers/h264_level_limits.cc)
(Table A-1 values), and the bundled FFmpeg's `-h filter=scale` / `-h filter=setdar`
help. Physical Wii/player testing is still required, especially for anamorphic
and custom output.
