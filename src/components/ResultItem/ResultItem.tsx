import { useEffect, useRef } from "react";
import type { CommandEntry } from "../../lib/types";
import { CommandIcon, HighlightedText, Kbd } from "../../ui";
import "./ResultItem.css";

interface ResultItemProps {
  command: CommandEntry;
  isSelected: boolean;
  onClick: () => void;
}

export function ResultItem({ command, isSelected, onClick }: ResultItemProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isSelected) {
      ref.current?.scrollIntoView({ block: "nearest" });
    }
  }, [isSelected]);

  return (
    <div
      ref={ref}
      className={`result-item ${isSelected ? "selected" : ""}`}
      onClick={onClick}
    >
      <CommandIcon
        id={command.id}
        category={command.category}
        iconDataUri={command.icon}
      />
      <div className="result-info">
        <div className="result-name">
          <HighlightedText
            text={command.name}
            indices={command.match_indices}
          />
        </div>
        <div className="result-desc">{command.description}</div>
      </div>
      <div className="result-shortcut">
        <Kbd>{"\u21B5"}</Kbd>
      </div>
    </div>
  );
}
