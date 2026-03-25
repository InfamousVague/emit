import { Toggle as BaseToggle } from "@base/primitives/toggle/Toggle";

interface ToggleProps {
  checked: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, disabled }: ToggleProps) {
  return (
    <BaseToggle
      checked={checked}
      onChange={(e: React.ChangeEvent<HTMLInputElement>) => onChange(e.target.checked)}
      disabled={disabled}
    />
  );
}
