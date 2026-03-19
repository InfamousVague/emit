import { useState, useEffect, useRef, useCallback } from "react";
import type { ParamDefinition, SelectOption } from "../../lib/types";
import { resolveParamOptions } from "../../lib/tauri";
import "./ParamWizard.css";

interface ParamInputProps {
  param: ParamDefinition;
  commandId: string;
  value: unknown;
  onChange: (value: unknown) => void;
  onSubmit: () => void;
  autoFocus?: boolean;
}

export function ParamInput({
  param,
  commandId,
  value,
  onChange,
  onSubmit,
  autoFocus = true,
}: ParamInputProps) {
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      onSubmit();
    }
  };

  switch (param.param_type.type) {
    case "Text":
    case "RichText":
    case "Url":
      return (
        <input
          type="text"
          className="param-text-input"
          value={(value as string) ?? ""}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={param.placeholder ?? ""}
          autoFocus={autoFocus}
        />
      );

    case "Number":
      return (
        <input
          type="number"
          className="param-text-input"
          value={(value as number) ?? ""}
          onChange={(e) => onChange(Number(e.target.value))}
          onKeyDown={handleKeyDown}
          placeholder={param.placeholder ?? ""}
          autoFocus={autoFocus}
        />
      );

    case "Boolean":
      return (
        <label className="param-toggle">
          <input
            type="checkbox"
            checked={(value as boolean) ?? false}
            onChange={(e) => onChange(e.target.checked)}
            onKeyDown={handleKeyDown}
          />
          <span className="param-toggle-label">{param.name}</span>
        </label>
      );

    case "Date":
      return (
        <input
          type="date"
          className="param-text-input"
          value={(value as string) ?? ""}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          autoFocus={autoFocus}
        />
      );

    case "Select":
      return (
        <SelectInput
          options={param.param_type.options}
          value={value as string}
          onChange={onChange}
          onSubmit={onSubmit}
          placeholder={param.placeholder ?? "Select..."}
          autoFocus={autoFocus}
        />
      );

    case "MultiSelect":
      return (
        <MultiSelectInput
          options={param.param_type.options}
          value={(value as string[]) ?? []}
          onChange={onChange}
          onSubmit={onSubmit}
          placeholder={param.placeholder ?? "Select..."}
          autoFocus={autoFocus}
        />
      );

    case "DatabasePicker":
    case "PagePicker":
    case "People":
    case "DynamicSelect":
      return (
        <AutocompleteInput
          commandId={commandId}
          paramId={param.id}
          value={value as string}
          onChange={onChange}
          onSubmit={onSubmit}
          placeholder={param.placeholder ?? "Search..."}
          autoFocus={autoFocus}
        />
      );

    default:
      return (
        <input
          type="text"
          className="param-text-input"
          value={(value as string) ?? ""}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={param.placeholder ?? ""}
          autoFocus={autoFocus}
        />
      );
  }
}

function SelectInput({
  options,
  value,
  onChange,
  onSubmit,
  placeholder,
  autoFocus,
}: {
  options: SelectOption[];
  value: string;
  onChange: (value: unknown) => void;
  onSubmit: () => void;
  placeholder: string;
  autoFocus: boolean;
}) {
  const [filter, setFilter] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);

  const filtered = options.filter(
    (o) =>
      !filter || o.label.toLowerCase().includes(filter.toLowerCase()),
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIdx((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIdx((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filtered[selectedIdx]) {
        onChange(filtered[selectedIdx].value);
        onSubmit();
      }
    }
  };

  return (
    <div className="param-select">
      <input
        type="text"
        className="param-text-input"
        value={filter}
        onChange={(e) => {
          setFilter(e.target.value);
          setSelectedIdx(0);
        }}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        autoFocus={autoFocus}
      />
      <div className="param-options-list">
        {filtered.map((opt, i) => (
          <div
            key={opt.value}
            className={`param-option ${i === selectedIdx ? "selected" : ""} ${value === opt.value ? "active" : ""}`}
            onClick={() => {
              onChange(opt.value);
              onSubmit();
            }}
          >
            {opt.color && (
              <span
                className="param-color-chip"
                style={{ backgroundColor: opt.color }}
              />
            )}
            {opt.label}
          </div>
        ))}
      </div>
    </div>
  );
}

function MultiSelectInput({
  options,
  value,
  onChange,
  onSubmit,
  placeholder,
  autoFocus,
}: {
  options: SelectOption[];
  value: string[];
  onChange: (value: unknown) => void;
  onSubmit: () => void;
  placeholder: string;
  autoFocus: boolean;
}) {
  const [filter, setFilter] = useState("");

  const filtered = options.filter(
    (o) =>
      !filter || o.label.toLowerCase().includes(filter.toLowerCase()),
  );

  const toggleOption = (val: string) => {
    const next = value.includes(val)
      ? value.filter((v) => v !== val)
      : [...value, val];
    onChange(next);
  };

  return (
    <div className="param-select">
      <div className="param-chips">
        {value.map((v) => {
          const opt = options.find((o) => o.value === v);
          return (
            <span
              key={v}
              className="param-chip"
              style={opt?.color ? { backgroundColor: opt.color } : undefined}
              onClick={() => toggleOption(v)}
            >
              {opt?.label ?? v} &times;
            </span>
          );
        })}
      </div>
      <input
        type="text"
        className="param-text-input"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            onSubmit();
          }
        }}
        placeholder={placeholder}
        autoFocus={autoFocus}
      />
      <div className="param-options-list">
        {filtered.map((opt) => (
          <div
            key={opt.value}
            className={`param-option ${value.includes(opt.value) ? "active" : ""}`}
            onClick={() => toggleOption(opt.value)}
          >
            {opt.color && (
              <span
                className="param-color-chip"
                style={{ backgroundColor: opt.color }}
              />
            )}
            {opt.label}
            {value.includes(opt.value) && <span className="param-check">✓</span>}
          </div>
        ))}
      </div>
    </div>
  );
}

function AutocompleteInput({
  commandId,
  paramId,
  value,
  onChange,
  onSubmit,
  placeholder,
  autoFocus,
}: {
  commandId: string;
  paramId: string;
  value: string;
  onChange: (value: unknown) => void;
  onSubmit: () => void;
  placeholder: string;
  autoFocus: boolean;
}) {
  const [query, setQuery] = useState("");
  const [options, setOptions] = useState<SelectOption[]>([]);
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [selectedLabel, setSelectedLabel] = useState("");
  const debounceRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const fetchOptions = useCallback(
    async (q: string) => {
      const results = await resolveParamOptions(commandId, paramId, q);
      setOptions(results);
      setSelectedIdx(0);
    },
    [commandId, paramId],
  );

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => fetchOptions(query), 200);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [query, fetchOptions]);

  // Initial load
  useEffect(() => {
    fetchOptions("");
  }, [fetchOptions]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIdx((i) => Math.min(i + 1, options.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIdx((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (options[selectedIdx]) {
        onChange(options[selectedIdx].value);
        setSelectedLabel(options[selectedIdx].label);
        onSubmit();
      }
    }
  };

  return (
    <div className="param-select">
      {selectedLabel && (
        <div className="param-selected-label">
          Selected: <strong>{selectedLabel}</strong>
        </div>
      )}
      <input
        type="text"
        className="param-text-input"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        autoFocus={autoFocus}
      />
      <div className="param-options-list">
        {options.map((opt, i) => (
          <div
            key={opt.value}
            className={`param-option ${i === selectedIdx ? "selected" : ""} ${value === opt.value ? "active" : ""}`}
            onClick={() => {
              onChange(opt.value);
              setSelectedLabel(opt.label);
              onSubmit();
            }}
          >
            {opt.color && (
              <span
                className="param-color-chip"
                style={{ backgroundColor: opt.color }}
              />
            )}
            {opt.label}
          </div>
        ))}
        {options.length === 0 && query.length > 0 && (
          <div className="param-option-empty">No results</div>
        )}
      </div>
    </div>
  );
}
