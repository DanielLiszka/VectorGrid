export const $ = (id) => document.getElementById(id);

export function setHintVisible(id, visible) {
  const el = $(id);
  if (!el) return;
  el.style.display = visible ? "block" : "none";
}
