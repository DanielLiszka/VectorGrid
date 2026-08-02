export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

export const SYSTEM_DARK_THEME_QUERY = "(prefers-color-scheme: dark)";

const THEME_PREFERENCES = new Set<ThemePreference>([
  "light",
  "dark",
  "system",
]);

interface ThemeRoot {
  dataset: DOMStringMap | Record<string, string>;
}

type MatchMedia = (query: string) => MediaQueryList;

interface ActiveThemeController {
  setPreference: (preference: ThemePreference) => void;
  dispose: () => void;
}

let activeThemeController: ActiveThemeController | null = null;

export function normalizeThemePreference(value: unknown): ThemePreference {
  const normalized = String(value ?? "")
    .trim()
    .toLowerCase() as ThemePreference;
  return THEME_PREFERENCES.has(normalized) ? normalized : "system";
}

function resolveMatchMedia(): MatchMedia | null {
  if (
    typeof window === "undefined" ||
    typeof window.matchMedia !== "function"
  ) {
    return null;
  }
  return (query: string) => window.matchMedia(query);
}

function resolveTheme(
  preference: ThemePreference,
  mediaQuery: MediaQueryList | null,
): ResolvedTheme {
  if (preference === "dark") return "dark";
  if (preference === "light") return "light";
  return mediaQuery?.matches ? "dark" : "light";
}

function setRootTheme(
  root: ThemeRoot,
  preference: ThemePreference,
  mediaQuery: MediaQueryList | null,
): void {
  root.dataset.themePref = preference;
  root.dataset.theme = resolveTheme(preference, mediaQuery);
}

function resolveDocumentRoot(): ThemeRoot | null {
  if (typeof document === "undefined") return null;
  return document.documentElement ?? null;
}

/**
 * Applies a theme preference immediately. Once the root controller is installed,
 * this also owns the lifecycle of the system color-scheme listener.
 */
export function applyRootThemePreference(value: unknown): ThemePreference {
  const preference = normalizeThemePreference(value);
  if (activeThemeController) {
    activeThemeController.setPreference(preference);
    return preference;
  }

  const root = resolveDocumentRoot();
  if (!root) return preference;

  const matchMedia = resolveMatchMedia();
  let mediaQuery: MediaQueryList | null = null;
  if (preference === "system" && matchMedia) {
    try {
      mediaQuery = matchMedia(SYSTEM_DARK_THEME_QUERY);
    } catch {
      mediaQuery = null;
    }
  }
  setRootTheme(root, preference, mediaQuery);
  return preference;
}

export interface RootThemeControllerOptions {
  root?: ThemeRoot;
  matchMedia?: MatchMedia | null;
}

/**
 * Installs the app-owned system theme controller and returns its teardown.
 * Reinstalling replaces the previous controller so at most one media listener
 * remains active.
 */
export function installRootThemeController(
  options: RootThemeControllerOptions = {},
): () => void {
  activeThemeController?.dispose();

  const root = options.root ?? resolveDocumentRoot();
  if (!root) return () => undefined;

  const matchMedia =
    options.matchMedia === undefined
      ? resolveMatchMedia()
      : options.matchMedia;
  let preference = normalizeThemePreference(root.dataset.themePref);
  let mediaQuery: MediaQueryList | null = null;
  let listening = false;
  let disposed = false;

  const handleSystemThemeChange = (): void => {
    if (disposed || preference !== "system") return;
    setRootTheme(root, preference, mediaQuery);
  };

  const stopListening = (): void => {
    if (!mediaQuery || !listening) {
      mediaQuery = null;
      listening = false;
      return;
    }
    try {
      if (typeof mediaQuery.removeEventListener === "function") {
        mediaQuery.removeEventListener("change", handleSystemThemeChange);
      } else {
        mediaQuery.removeListener?.(handleSystemThemeChange);
      }
    } catch {
      // Theme observation is optional; teardown must never block app cleanup.
    }
    mediaQuery = null;
    listening = false;
  };

  const startListening = (): void => {
    if (!matchMedia) return;
    try {
      mediaQuery = matchMedia(SYSTEM_DARK_THEME_QUERY);
    } catch {
      mediaQuery = null;
      return;
    }
    try {
      if (typeof mediaQuery.addEventListener === "function") {
        mediaQuery.addEventListener("change", handleSystemThemeChange);
      } else if (typeof mediaQuery.addListener === "function") {
        mediaQuery.addListener(handleSystemThemeChange);
      } else {
        mediaQuery = null;
        return;
      }
      listening = true;
    } catch {
      mediaQuery = null;
      listening = false;
    }
  };

  const controller: ActiveThemeController = {
    setPreference(nextPreference) {
      if (disposed) return;
      stopListening();
      preference = normalizeThemePreference(nextPreference);
      if (preference === "system") startListening();
      setRootTheme(root, preference, mediaQuery);
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      stopListening();
      if (activeThemeController === controller) {
        activeThemeController = null;
      }
    },
  };

  activeThemeController = controller;
  controller.setPreference(preference);
  return controller.dispose;
}
