import { useEffect, useRef } from "react";
import { ShieldWarning } from "@phosphor-icons/react";
import { wmCheckAccessibility, wmRequestAccessibility } from "../../lib/tauri";

interface Props {
  onGranted: () => void;
}

export function AccessibilityPrompt({ onGranted }: Props) {
  const polling = useRef<ReturnType<typeof setInterval>>(undefined);

  useEffect(() => {
    polling.current = setInterval(async () => {
      const granted = await wmCheckAccessibility();
      if (granted) {
        clearInterval(polling.current);
        onGranted();
      }
    }, 2000);
    return () => clearInterval(polling.current);
  }, [onGranted]);

  return (
    <div className="wm-accessibility">
      <ShieldWarning size={32} weight="duotone" />
      <p className="wm-accessibility-title">Accessibility Permission Required</p>
      <p className="wm-accessibility-desc">
        Window Management needs Accessibility access to move and resize windows.
        Grant access in System Settings, then it will activate automatically.
      </p>
      <button
        className="wm-accessibility-btn"
        onClick={() => wmRequestAccessibility()}
      >
        Open System Settings
      </button>
    </div>
  );
}
