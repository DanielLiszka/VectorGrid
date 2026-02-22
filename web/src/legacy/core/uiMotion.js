let UI_REDUCED_MOTION_MQL = null;
let UI_REDUCED_MOTION_LISTENER_ATTACHED = false;

function prefersReducedMotion() {
  if (typeof window.matchMedia !== "function") return false;
  if (!UI_REDUCED_MOTION_MQL) UI_REDUCED_MOTION_MQL = window.matchMedia("(prefers-reduced-motion: reduce)");
  return !!UI_REDUCED_MOTION_MQL.matches;
}

function supportsViewTransitions() {
  return typeof document.startViewTransition === "function";
}

function syncUiTransitionClass() {
  const root = document.documentElement;
  if (!root) return;
  root.classList.toggle("ui-transitions", !prefersReducedMotion());
}

export function initUiMotionPrefs() {
  syncUiTransitionClass();
  if (UI_REDUCED_MOTION_LISTENER_ATTACHED) return;
  if (typeof window.matchMedia !== "function") return;
  if (!UI_REDUCED_MOTION_MQL) UI_REDUCED_MOTION_MQL = window.matchMedia("(prefers-reduced-motion: reduce)");
  const onChange = () => syncUiTransitionClass();
  try {
    if (UI_REDUCED_MOTION_MQL.addEventListener) UI_REDUCED_MOTION_MQL.addEventListener("change", onChange);
    else UI_REDUCED_MOTION_MQL.addListener(onChange);
    UI_REDUCED_MOTION_LISTENER_ATTACHED = true;
  } catch {}
}

export function runUiTransition(updateFn, opts) {
  if (typeof updateFn !== "function") return;
  const animate = opts?.animate === true;
  if (!animate || document.hidden || prefersReducedMotion()) {
    updateFn();
    return;
  }
  if (supportsViewTransitions()) {
    try {
      document.startViewTransition(() => {
        updateFn();
      });
      return;
    } catch {}
  }
  updateFn();
}
