import { useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getSettings } from "../lib/tauri";

interface UpdateState {
  available: boolean;
  version: string | null;
  downloading: boolean;
  dismissed: boolean;
}

export interface UseAutoUpdateReturn {
  update: UpdateState;
  installUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  checkNow: () => Promise<void>;
}

const POLL_INTERVAL_MS = 30 * 60 * 1000; // 30 minutes

export function useAutoUpdate(): UseAutoUpdateReturn {
  const [state, setState] = useState<UpdateState>({
    available: false,
    version: null,
    downloading: false,
    dismissed: false,
  });
  const updateRef = useRef<Update | null>(null);

  useEffect(() => {
    let cancelled = false;
    let intervalId: ReturnType<typeof setInterval>;

    async function checkOnce() {
      // Already found an update — skip further checks
      if (updateRef.current) return;

      try {
        const settings = await getSettings();
        if (!settings.check_for_updates) return;
      } catch {
        return;
      }

      try {
        const update = await check();
        if (update && !cancelled) {
          updateRef.current = update;
          setState((prev) => ({
            ...prev,
            available: true,
            version: update.version,
          }));
        }
      } catch (e) {
        console.warn("Update check failed:", e);
      }
    }

    checkOnce();
    intervalId = setInterval(checkOnce, POLL_INTERVAL_MS);

    return () => {
      cancelled = true;
      clearInterval(intervalId);
    };
  }, []);

  const installUpdate = async () => {
    if (!updateRef.current) return;
    setState((prev) => ({ ...prev, downloading: true }));
    try {
      await updateRef.current.downloadAndInstall();
      await relaunch();
    } catch (e) {
      console.warn("Update install failed:", e);
      setState((prev) => ({ ...prev, downloading: false }));
    }
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
          available: true,
          version: update.version,
          dismissed: false,
        }));
      }
    } catch (e) {
      console.warn("Update check failed:", e);
    }
  };

  return { update: state, installUpdate, dismissUpdate, checkNow };
}
