import { useEffect, useRef } from "react";
import type { CommandDefinition } from "../../lib/types";
import { Kbd } from "../../ui";
import "./CommandMode.css";

interface CommandModeProps {
  commands: CommandDefinition[];
  selectedIndex: number;
  onItemClick: (index: number) => void;
}

export function CommandMode({
  commands,
  selectedIndex,
  onItemClick,
}: CommandModeProps) {
  if (commands.length === 0) {
    return (
      <div className="command-mode-empty">
        <p>No commands found</p>
      </div>
    );
  }

  return (
    <div className="command-mode">
      {commands.map((cmd, i) => (
        <CommandItem
          key={cmd.id}
          command={cmd}
          isSelected={i === selectedIndex}
          onClick={() => onItemClick(i)}
        />
      ))}
    </div>
  );
}

function CommandItem({
  command,
  isSelected,
  onClick,
}: {
  command: CommandDefinition;
  isSelected: boolean;
  onClick: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isSelected) {
      ref.current?.scrollIntoView({ block: "nearest" });
    }
  }, [isSelected]);

  const categoryBadge =
    command.category === "Write" ? "cmd-badge-write" : "cmd-badge-read";

  return (
    <div
      ref={ref}
      className={`command-item ${isSelected ? "selected" : ""}`}
      onClick={onClick}
    >
      <div className={`cmd-badge ${categoryBadge}`}>
        {command.category === "Write" ? "W" : "R"}
      </div>
      <div className="command-info">
        <div className="command-name">
          <span className="command-extension">{command.extension_id}:</span>{" "}
          {command.name}
        </div>
        <div className="command-desc">{command.description}</div>
      </div>
      <div className="command-meta">
        {command.shortcut && <Kbd>{command.shortcut}</Kbd>}
        {command.requires_confirmation && (
          <span className="cmd-confirm-badge" title="Requires confirmation">
            !
          </span>
        )}
        <Kbd>{"\u21B5"}</Kbd>
      </div>
    </div>
  );
}
