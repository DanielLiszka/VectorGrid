import { $ } from "./dom.js";

let LAST_TOAST_AT_MS = 0;
let LAST_TOAST_MSG = "";

function ensureToastHost() {
  const existing = $("toastHost");
  if (existing) return existing;
  const host = document.createElement("div");
  host.id = "toastHost";
  host.className = "toastHost";
  host.setAttribute("aria-live", "polite");
  host.setAttribute("aria-relevant", "additions");
  host.setAttribute("aria-atomic", "false");
  document.body.appendChild(host);
  return host;
}

function removeToast(el) {
  if (!el) return;
  try { el.classList.remove("in"); } catch {}
  setTimeout(() => {
    try { el.remove(); } catch {}
  }, 220);
}

export function showToast(msg, kind, opts) {
  let text = String(msg || "").trim();
  if (!text) return;
  const maxCharsRaw = Number(opts?.maxChars ?? 240);
  const maxChars = Number.isFinite(maxCharsRaw) ? Math.max(80, Math.min(800, Math.floor(maxCharsRaw))) : 240;
  if (text.length > maxChars) {
    text = text.slice(0, Math.max(0, maxChars - 1)).trimEnd() + "…";
  }
  const k = String(kind || "").trim();

  const host = ensureToastHost();

  const toast = document.createElement("div");
  toast.className = `toast ${k}`.trim();

  const dot = document.createElement("div");
  dot.className = "toastDot";

  const body = document.createElement("div");
  body.className = "toastBody";
  body.textContent = text;

  const close = document.createElement("button");
  close.className = "toastClose";
  close.type = "button";
  close.title = "Dismiss";
  close.setAttribute("aria-label", "Dismiss");
  close.textContent = "×";
  close.addEventListener("click", () => removeToast(toast));

  toast.appendChild(dot);
  toast.appendChild(body);
  toast.appendChild(close);
  host.prepend(toast);

  while (host.children.length > 4) {
    const last = host.lastElementChild;
    if (!last) break;
    last.remove();
  }

  requestAnimationFrame(() => {
    try { toast.classList.add("in"); } catch {}
  });

  const ms = Number(opts?.ms || 0);
  const ttl = Number.isFinite(ms) && ms > 0 ? ms : (k === "danger" ? 6500 : 3800);
  setTimeout(() => removeToast(toast), ttl);
}

export function maybeToastFromStatus(msg, kind) {
  const k = String(kind || "").trim();
  if (k !== "ok" && k !== "danger") return;

  const text = String(msg || "").trim();
  if (!text) return;
  if (text.includes("...")) return;

  const now = Date.now();
  if (!Number.isFinite(now)) return;
  if ((now - LAST_TOAST_AT_MS) < 1200) return;
  if (text === LAST_TOAST_MSG) return;

  LAST_TOAST_AT_MS = now;
  LAST_TOAST_MSG = text;
  showToast(text, k);
}
