export function performanceLabel(percent: number | null | undefined, fps: number | null | undefined, speed: string | number | null | undefined, eta: number | null | undefined, paused = false): string {
  if (paused) return "Paused";
  const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v) && v >= 0;
  const rate = typeof speed === "string" ? Number(speed.replace(/x$/, "")) : speed;
  const seconds = finite(eta) ? Math.ceil(eta) : null;
  return `${finite(percent) ? Math.floor(Math.min(100, percent)) + "%" : "Measuring"}  •  ${finite(rate) && rate > 0 ? rate.toFixed(2) + "x" : "--"} Speed  •  ${finite(fps) ? fps.toFixed(1) : "--"} FPS  •  ETA: ${seconds === null ? "--" : `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`} remaining`;
}
