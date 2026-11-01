import keroro from "./assets/keroro-icon.webp";
import tamama from "./assets/tamama-icon.webp";
import giroro from "./assets/giroro-icon.webp";
import kururu from "./assets/kururu-icon.webp";
import dororo from "./assets/dororo-icon.webp";
import { getPreferences, savePreferences, isTheme, type Theme } from "./preferences";

const icons = { keroro, tamama, giroro, kururu, dororo } as const;

export function initializeThemes(onSelect: (theme: Theme) => void): void {
  const buttons = document.querySelectorAll<HTMLButtonElement>(".platoon-btn[data-theme]");

  function selectTheme(theme: Theme): void {
    document.body.dataset.theme = theme;
    buttons.forEach((button) => {
      const selected = button.dataset.theme === theme;
      button.setAttribute("aria-pressed", String(selected));
    });
  }

  buttons.forEach((button) => {
    const theme = button.dataset.theme;
    const image = button.querySelector<HTMLImageElement>(".icon-wrapper > img");
    if (!isTheme(theme) || !image) {
      throw new Error("Invalid character theme button markup.");
    }
    image.src = icons[theme];
    button.addEventListener("click", () => {
      selectTheme(theme);
      onSelect(theme);
      savePreferences({ selectedTheme: theme });
    });
  });

  selectTheme(getPreferences().selectedTheme);
}
