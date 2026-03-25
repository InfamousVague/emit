import { useEffect, useRef, useState, useCallback } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getSettings } from "../lib/tauri";

type UpdatePhase = "idle" | "available" | "downloading" | "ready" | "error";

interface UpdateState {
  phase: UpdatePhase;
  version: string | null;
  progress: number;
  dismissed: boolean;
  error: string | null;
}

export interface UseAutoUpdateReturn {
  update: UpdateState;
  cancelDownload: () => void;
  relaunchApp: () => Promise<void>;
  dismissUpdate: () => void;
  checkNow: () => Promise<void>;
}

const POLL_INTERVAL_MS = 30 * 60 * 1000; // 30 minutes

export function useAutoUpdate(): UseAutoUpdateReturn {
  const [state, setState] = useState<UpdateState>({
    phase: "idle",
    version: null,
    progress: 0,
    dismissed: false,
    error: null,
  });
  const updateRef = useRef<Update | null>(null);
  const cancelledRef = useRef(false);

  const startDownload = useCallback(async () => {
    const update = updateRef.current;
    if (!update) return;

    cancelledRef.current = false;
    setState((prev) => ({ ...prev, phase: "downloading", progress: 0, error: null }));

    let contentLength = 0;
    let downloaded = 0;

    try {
      await update.downloadAndInstall((event) => {
        if (cancelledRef.current) return;

        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress": {
            downloaded += event.data.chunkLength;
            const pct = contentLength > 0 ? Math.min((downloaded / contentLength) * 100, 100) : 0;
            setState((prev) => ({ ...prev, progress: pct }));
            break;
          }
          case "Finished":
            setState((prev) => ({ ...prev, phase: "ready", progress: 100 }));
            break;
        }
      });

      if (!cancelledRef.current) {
        setState((prev) => ({ ...prev, phase: "ready", progress: 100 }));
      }
    } catch (e) {
      if (!cancelledRef.current) {
        console.warn("Update download failed:", e);
        setState((prev) => ({ ...prev, phase: "error", error: String(e) }));
      }
    }
  }, []);

  useEffect(() => {
    let unmounted = false;
    let intervalId: ReturnType<typeof setInterval>;

    async function checkOnce() {
      if (import.meta.env.DEV) return;
      if (updateRef.current) return;

      try {
        const settings = await getSettings();
        if (!settings.check_for_updates) return;
      } catch {
        return;
      }

      try {
        const update = await check();
        if (update && !unmounted) {
          updateRef.current = update;
          setState((prev) => ({
            ...prev,
            phase: "available",
            version: update.version,
          }));
          // Auto-download immediately
          startDownload();
        }
      } catch (e) {
        console.warn("Update check failed:", e);
      }
    }

    checkOnce();
    intervalId = setInterval(checkOnce, POLL_INTERVAL_MS);

    return () => {
      unmounted = true;
      clearInterval(intervalId);
    };
  }, [startDownload]);

  const cancelDownload = () => {
    cancelledRef.current = true;
    setState((prev) => ({ ...prev, phase: "available", progress: 0 }));
  };

  const relaunchApp = async () => {
    await relaunch();
  };

  const dismissUpdate = () => {
    setState((prev) => ({ ...prev, dismissed: true }));
  };

  const checkNow = async () => {
    if (updateRef.current) return;
    try {
      const update = await check();
      if (update) {
        updateRef.current = update;
        setState((prev) => ({
          ...prev,
          phase: "available",
          version: update.version,
          dismissed: false,
        }));
        startDownload();
      }
    } catch (e) {
      console.warn("Update check failed:", e);
    }
  };

  return { update: state, cancelDownload, relaunchApp, dismissUpdate, checkNow };
}
