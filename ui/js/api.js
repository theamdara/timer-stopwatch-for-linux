// Thin bridge around the Tauri v2 global API. The app ships without a
// bundler, so `withGlobalTauri` is enabled in tauri.conf.json.

const tauri = typeof window !== "undefined" ? window.__TAURI__ : undefined;

export const hasTauri = Boolean(tauri?.core?.invoke);

export async function invoke(command, args = {}) {
  if (!hasTauri) {
    throw new Error("The Tauri bridge is unavailable. Start Catime with `cargo tauri dev`.");
  }
  return tauri.core.invoke(command, args);
}

export async function listen(eventName, handler) {
  if (!hasTauri) return () => {};
  const unlisten = await tauri.event.listen(eventName, (event) => handler(event.payload));
  return unlisten;
}
