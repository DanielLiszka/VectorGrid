export function fileBasename(path) {
  const s = String(path || "");
  if (!s) return "";
  const parts = s.split(/[\\/]/g).filter((p) => !!p);
  return parts.length ? parts[parts.length - 1] : s;
}
