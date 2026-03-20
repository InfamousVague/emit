import { useState, useCallback, useEffect, useRef } from "react";

interface ShortcutRecorderProps {
  value: string;
  onChange: (keys: string) => void;
  conflict?: string;
}

export function ShortcutRecorder({
  value,
  onChange,
  conflict,
}: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);
  const [pendingKeys, setPendingKeys] = useState<string | null>(null);
  const ref = useRef<HTMLButtonElement>(null);

  const startRecording = useCallback(() => {
    setRecording(true);
    setPendingKeys(null);
  }, []);

  const stopRecording = useCallback(() => {
    setRecording(false);
    if (pendingKeys) {
      onChange(pendingKeys);
    }
    setPendingKeys(null);
  }, [pendingKeys, onChange]);

  useEffect(() => {
    if (!recording) return;

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Ignore lone modifier presses
      if (
        ["Shift", "Control", "Alt", "Meta", "Command"].includes(e.key)
      ) {
        return;
      }

      if (e.key === "Escape") {
        setRecording(false);
        setPendingKeys(null);
        return;
      }

      const parts: string[] = [];
      if (e.shiftKey) parts.push("Shift");
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.metaKey) parts.push("Cmd");

      // Convert key to display format
      let key = e.key;
      if (key.length === 1) {
        key = key.toUpperCase();
      } else {
        // Map special keys
        const keyMap: Record<string, string> = {
          ArrowUp: "Up",
          ArrowDown: "Down",
          ArrowLeft: "Left",
          ArrowRight: "Right",
          " ": "Space",
        };
        key = keyMap[key] || key;
      }

      parts.push(key);
      const combo = parts.join("+");
      setPendingKeys(combo);

      // Auto-save after brief delay
      setTimeout(() => {
        onChange(combo);
        setRecording(false);
        setPendingKeys(null);
      }, 200);
    };

    document.addEventListener("keydown", handler, true);
    return () => document.removeEventListener("keydown", handler, true);
  }, [recording, onChange]);

  // Click outside to cancel recording
  useEffect(() => {
    if (!recording) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setRecording(false);
        setPendingKeys(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [recording]);

  const displayValue = recording
    ? pendingKeys || "Press keys…"
    : value;

  return (
    <button
      ref={ref}
      className={`shortcut-recorder ${recording ? "shortcut-recorder--recording" : ""} ${conflict ? "shortcut-recorder--conflict" : ""}`}
      onClick={startRecording}
      title={conflict ? `Conflicts with: ${conflict}` : "Click to change shortcut"}
    >
      <span className="shortcut-recorder__keys">
        {displayValue.split("+").map((part, i) => (
          <kbd key={i} className="shortcut-recorder__key">
            {part === "Cmd" ? "⌘" : part === "Shift" ? "⇧" : part === "Alt" ? "⌥" : part === "Ctrl" ? "⌃" : part}
          </kbd>
        ))}
      </span>
      {recording && (
        <span className="shortcut-recorder__hint">ESC to cancel</span>
      )}
    </button>
  );
}
