import { Select as BaseSelect } from "@base/primitives/select/Select";

interface SelectOption {
  value: string | number;
  label: string;
}

interface SelectProps {
  value: string | number;
  options: SelectOption[];
  onChange: (value: string) => void;
  variant?: "default" | "pill";
}

export function Select({ value, options, onChange, variant = "default" }: SelectProps) {
  return (
    <BaseSelect
      value={value}
      onChange={(e: React.ChangeEvent<HTMLSelectElement>) => onChange(e.target.value)}
      shape={variant === "pill" ? "pill" : "default"}
      size="sm"
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </BaseSelect>
  );
}
