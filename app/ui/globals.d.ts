// What the page finds on window: Tauri's API (withGlobalTauri in tauri.conf.json), only inside the
// app, and the extra face sets once vendor/faces.js has loaded. Only the parts the UI uses.

type Filter = { name: string; extensions: string[] };

interface Window {
  __TAURI__?: {
    core: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>; convertFileSrc(path: string): string };
    app?: { getVersion(): Promise<string> };
    window?: { getCurrentWindow(): { setTheme(t: "light" | "dark" | null): Promise<void>; setBadgeCount(n?: number): Promise<void> } };
    dialog?: { open(o: { multiple?: boolean; directory?: boolean; title?: string; defaultPath?: string; filters?: Filter[] }): Promise<string | string[] | null> };
    updater?: { check(): Promise<{ version: string; downloadAndInstall(): Promise<void> } | null> };
    process?: { relaunch(): Promise<void> };
    event?: { listen(name: string, handler: (e: { payload: string }) => void): Promise<() => void> };
    opener?: { revealItemInDir(path: string): Promise<void> };
    notification?: { isPermissionGranted(): Promise<boolean>; requestPermission(): Promise<string>; sendNotification(n: { title: string; body: string }): void };
  };
  Faces?: (set: string, seed: string) => string;
}
