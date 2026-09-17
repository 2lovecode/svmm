/** Apply settings.theme (`system` | `light` | `dark`) to documentElement. */

export type ThemePreference = "system" | "light" | "dark";

export function resolveTheme(preference: string): "light" | "dark" {
  if (preference === "light" || preference === "dark") {
    return preference;
  }
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return "light";
}

export function applyTheme(preference: string): void {
  if (typeof document === "undefined") return;
  const resolved = resolveTheme(preference);
  const root = document.documentElement;
  root.setAttribute("data-theme", resolved);
  root.classList.toggle("theme-dark", resolved === "dark");
  root.classList.toggle("theme-light", resolved === "light");
}

let mediaListener: ((e: MediaQueryListEvent) => void) | null = null;
let currentPreference = "system";

export function bindTheme(preference: string): void {
  currentPreference = preference || "system";
  applyTheme(currentPreference);

  if (typeof window === "undefined" || !window.matchMedia) return;

  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  if (mediaListener) {
    mq.removeEventListener("change", mediaListener);
    mediaListener = null;
  }
  if (currentPreference === "system") {
    mediaListener = () => applyTheme("system");
    mq.addEventListener("change", mediaListener);
  }
}
