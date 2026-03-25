import { useEffect, useRef } from "react";
import { Icon } from "@base/primitives/icon/Icon";
import { arrowLeft, search } from "../../lib/icons";
import { Kbd } from "../../ui";
import type { LabelRange } from "../../hooks/useCommandParser";
import "./SearchInput.css";

interface SearchInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  onBack?: () => void;
  trailing?: React.ReactNode;
  onKeyDown?: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  ghostText?: string;
  readOnly?: boolean;
  labelRanges?: LabelRange[];
  /** If set, moves cursor to this position after value update */
  cursorPos?: number | null;
  onCursorApplied?: () => void;
}

function renderStyledText(text: string, ranges: LabelRange[]) {
  const segments: React.ReactNode[] = [];
  let pos = 0;

  for (const range of ranges) {
    if (pos < range.start) {
      segments.push(<span key={`t${pos}`}>{text.slice(pos, range.start)}</span>);
    }
    segments.push(
      <span key={`l${range.start}`} className="search-label-dim">
        {text.slice(range.start, range.end)}
      </span>,
    );
    pos = range.end;
  }

  if (pos < text.length) {
    segments.push(<span key={`t${pos}`}>{text.slice(pos)}</span>);
  }

  return <>{segments}</>;
}

export function SearchInput({ value, onChange, placeholder, onBack, trailing, onKeyDown, ghostText, readOnly, labelRanges, cursorPos, onCursorApplied }: SearchInputProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const hasLabels = labelRanges && labelRanges.length > 0;

  useEffect(() => {
    if (!readOnly) inputRef.current?.focus();
  }, [readOnly]);

  // Apply pending cursor position after value update
  useEffect(() => {
    if (cursorPos != null && inputRef.current) {
      requestAnimationFrame(() => {
        inputRef.current?.setSelectionRange(cursorPos, cursorPos);
        onCursorApplied?.();
      });
    }
  }, [cursorPos, onCursorApplied]);

  return (
    <div className="search-bar">
      {onBack ? (
        <button className="search-back" onClick={onBack} aria-label="Back">
          <Icon icon={arrowLeft} size="sm" />
        </button>
      ) : (
        <Icon icon={search} size="sm" />
      )}
      <div className="search-input-wrapper">
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={placeholder ?? "Search for apps and commands..."}
          spellCheck={false}
          readOnly={readOnly}
          className={hasLabels ? "has-labels" : undefined}
        />
        {hasLabels && (
          <span className="search-text-overlay" aria-hidden>
            {renderStyledText(value, labelRanges)}
          </span>
        )}
        {ghostText && (
          <span className="search-ghost" aria-hidden>
            <span className="search-ghost-hidden">{value}</span>
            {ghostText}
          </span>
        )}
      </div>
      {trailing && <div className="search-trailing">{trailing}</div>}
      <Kbd>esc</Kbd>
    </div>
  );
}
