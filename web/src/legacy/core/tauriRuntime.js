export function tauriInvoke() {
  const t = window.__TAURI__ || {};
  return t?.core?.invoke || t?.invoke || null;
}

export function tauriListen() {
  const t = window.__TAURI__ || {};
  return t?.event?.listen || null;
}
