import { useEffect, useState } from "react";
import { undoLastAction } from "../../lib/tauri";
import { Button } from "../../ui";

interface UndoToastProps {
  message: string;
  onDismiss: () => void;
}

export function UndoToast({ message, onDismiss }: UndoToastProps) {
  const [isUndoing, setIsUndoing] = useState(false);

  useEffect(() => {
    const timer = setTimeout(onDismiss, 5000);
    return () => clearTimeout(timer);
  }, [onDismiss]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "z") {
        e.preventDefault();
        handleUndo();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const handleUndo = async () => {
    setIsUndoing(true);
    try {
      await undoLastAction();
    } catch {
      // notification will show error
    }
    onDismiss();
  };

  return (
    <div className="undo-toast">
      <span className="undo-message">{message}</span>
      <Button
        variant="primary"
        size="sm"
        onClick={handleUndo}
        disabled={isUndoing}
      >
        {isUndoing ? "Undoing..." : "Undo (⌘Z)"}
      </Button>
    </div>
  );
}
