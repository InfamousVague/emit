import { useEffect } from "react";
import type { CommandEntry } from "../lib/types";
import { hideWindow, quickLook, revealInFinder } from "../lib/tauri";

interface UseKeyboardNavOptions {
  results: CommandEntry[];
  selectedIndex: number;
  setSelectedIndex: (i: number | ((prev: number) => number)) => void;
  onExecute: (id: string) => Promise<void>;
  disabled?: boolean;
}

export function useKeyboardNav({
  results,
  selectedIndex,
  setSelectedIndex,
  onExecute,
  disabled = false,
}: UseKeyboardNavOptions) {
  useEffect(() => {
    if (disabled) return;

    const handler = async (e: KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          break;
        case "Enter":
          if (results[selectedIndex]) {
            // Cmd+Shift+Enter → Reveal in Finder for file results
            if (e.metaKey && e.shiftKey && results[selectedIndex].id.startsWith("file.")) {
              e.preventDefault();
              const path = results[selectedIndex].id.slice(5).split(":").join("/");
              revealInFinder(path);
              break;
            }
            await onExecute(results[selectedIndex].id);
          }
          break;
        case " ":
          // Shift+Space → Quick Look for file results
          if (e.shiftKey && results[selectedIndex]?.id.startsWith("file.")) {
            e.preventDefault();
            const path = results[selectedIndex].id.slice(5).split(":").join("/");
            quickLook(path);
          }
          break;
        case "Escape":
          await hideWindow();
          break;
      }
    };

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [results, selectedIndex, setSelectedIndex, onExecute, disabled]);
}
