import "./Select.css";

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
    <select
      className={`emit-select ${variant === "pill" ? "emit-select--pill" : ""}`}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}
