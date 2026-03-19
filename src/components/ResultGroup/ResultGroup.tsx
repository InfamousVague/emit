import type { CommandEntry } from "../../lib/types";
import { ResultItem } from "../ResultItem/ResultItem";
import "./ResultGroup.css";

interface ResultGroupProps {
  category: string;
  commands: CommandEntry[];
  selectedIndex: number;
  globalOffset: number;
  onItemClick: (index: number) => void;
}

export function ResultGroup({
  category,
  commands,
  selectedIndex,
  globalOffset,
  onItemClick,
}: ResultGroupProps) {
  return (
    <div className="result-group">
      <div className="result-section">{category}</div>
      {commands.map((cmd, i) => {
        const globalIndex = globalOffset + i;
        return (
          <ResultItem
            key={cmd.id}
            command={cmd}
            isSelected={globalIndex === selectedIndex}
            onClick={() => onItemClick(globalIndex)}
          />
        );
      })}
    </div>
  );
}
