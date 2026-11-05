import { invoke } from "./ipc";
import { encoderOptions, type HardwareEncoder } from "./hardware";

let detected: Promise<readonly string[]> | undefined;
export async function populateEncoders(select: HTMLSelectElement, value: HardwareEncoder): Promise<void> {
  select.replaceChildren(new Option("Auto (best available GPU)", "auto"), new Option("Software x264 (Default)", "x264"));
  if (value !== "auto" && value !== "x264") select.add(new Option(`${encoderOptions.find(e => e.value === value)?.label} (checking…)`, value));
  select.value = value;
  select.title = select.selectedOptions[0]?.textContent ?? "";
  detected ??= invoke("get_available_encoders").catch(error => { detected = undefined; throw error; });
  const codecs = await detected;
  const chosen = select.value;
  const options = [new Option("Auto (best available GPU)", "auto"), ...encoderOptions.map(encoder => {
    const available = codecs.includes(encoder.codec);
    const label = encoder.value === "x264" ? encoder.label : `${encoder.label} (${available ? "Detected" : "Unavailable — CPU fallback"})`;
    const option = new Option(label, encoder.value); option.disabled = !available && encoder.value !== value; return option;
  })];
  select.replaceChildren(...options); select.value = chosen;
  select.title = select.selectedOptions[0]?.textContent ?? "";
}
