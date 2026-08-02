import { afterEach, describe, expect, it, vi } from "vitest";
import {
  applyRootThemePreference,
  installRootThemeController,
  normalizeThemePreference,
  SYSTEM_DARK_THEME_QUERY,
} from "./themeController";

interface MediaQueryHarness {
  mediaQuery: MediaQueryList;
  emit: (matches: boolean) => void;
  addEventListener: ReturnType<typeof vi.fn>;
  removeEventListener: ReturnType<typeof vi.fn>;
}

function createMediaQueryHarness(initialMatches: boolean): MediaQueryHarness {
  let matches = initialMatches;
  const listeners = new Set<() => void>();
  const addEventListener = vi.fn(
    (_type: string, listener: EventListenerOrEventListenerObject) => {
      if (typeof listener === "function") listeners.add(listener as () => void);
    },
  );
  const removeEventListener = vi.fn(
    (_type: string, listener: EventListenerOrEventListenerObject) => {
      if (typeof listener === "function")
        listeners.delete(listener as () => void);
    },
  );
  const mediaQuery = {
    get matches() {
      return matches;
    },
    media: SYSTEM_DARK_THEME_QUERY,
    onchange: null,
    addEventListener,
    removeEventListener,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  } as unknown as MediaQueryList;

  return {
    mediaQuery,
    addEventListener,
    removeEventListener,
    emit(nextMatches) {
      matches = nextMatches;
      for (const listener of listeners) listener();
    },
  };
}

let stopController: (() => void) | null = null;

afterEach(() => {
  stopController?.();
  stopController = null;
});

describe("root theme controller", () => {
  it("normalizes missing and unsupported preferences to system", () => {
    expect(normalizeThemePreference(undefined)).toBe("system");
    expect(normalizeThemePreference(" MIDNIGHT ")).toBe("system");
    expect(normalizeThemePreference(" Dark ")).toBe("dark");
  });

  it("tracks system changes only while system mode is active", () => {
    const root = {
      dataset: { themePref: "system" } as Record<string, string>,
    };
    const media = createMediaQueryHarness(false);
    const matchMedia = vi.fn(() => media.mediaQuery);

    stopController = installRootThemeController({ root, matchMedia });

    expect(matchMedia).toHaveBeenCalledWith(SYSTEM_DARK_THEME_QUERY);
    expect(media.addEventListener).toHaveBeenCalledTimes(1);
    expect(root.dataset).toMatchObject({
      themePref: "system",
      theme: "light",
    });

    media.emit(true);
    expect(root.dataset.theme).toBe("dark");

    applyRootThemePreference("light");
    expect(root.dataset).toMatchObject({
      themePref: "light",
      theme: "light",
    });
    expect(media.removeEventListener).toHaveBeenCalledTimes(1);

    media.emit(true);
    expect(root.dataset.theme).toBe("light");

    applyRootThemePreference("system");
    expect(media.addEventListener).toHaveBeenCalledTimes(2);
    expect(root.dataset.theme).toBe("dark");
  });

  it("removes the system listener on app teardown", () => {
    const root = {
      dataset: { themePref: "system" } as Record<string, string>,
    };
    const media = createMediaQueryHarness(false);
    stopController = installRootThemeController({
      root,
      matchMedia: () => media.mediaQuery,
    });

    stopController();
    stopController = null;
    media.emit(true);

    expect(media.removeEventListener).toHaveBeenCalledTimes(1);
    expect(root.dataset.theme).toBe("light");
  });

  it("replaces an existing controller without leaving duplicate listeners", () => {
    const firstRoot = {
      dataset: { themePref: "system" } as Record<string, string>,
    };
    const firstMedia = createMediaQueryHarness(false);
    const stopFirst = installRootThemeController({
      root: firstRoot,
      matchMedia: () => firstMedia.mediaQuery,
    });
    const secondRoot = {
      dataset: { themePref: "system" } as Record<string, string>,
    };
    const secondMedia = createMediaQueryHarness(true);

    stopController = installRootThemeController({
      root: secondRoot,
      matchMedia: () => secondMedia.mediaQuery,
    });
    stopFirst();

    expect(firstMedia.removeEventListener).toHaveBeenCalledTimes(1);
    expect(secondMedia.addEventListener).toHaveBeenCalledTimes(1);
    expect(secondRoot.dataset.theme).toBe("dark");
  });
});
