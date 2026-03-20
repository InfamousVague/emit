import type { CommandDefinition } from "../../lib/types";
import { Button } from "../../ui";

interface FollowUpBarProps {
  followUpIds: string[];
  allCommands: CommandDefinition[];
  onSelect: (command: CommandDefinition) => void;
}

export function FollowUpBar({
  followUpIds,
  allCommands,
  onSelect,
}: FollowUpBarProps) {
  const followUps = followUpIds
    .map((id) => allCommands.find((c) => c.id === id))
    .filter((c): c is CommandDefinition => c != null);

  if (followUps.length === 0) return null;

  return (
    <div className="follow-up-bar">
      <span className="follow-up-label">Next:</span>
      {followUps.map((cmd) => (
        <Button
          key={cmd.id}
          size="sm"
          onClick={() => onSelect(cmd)}
        >
          {cmd.name}
        </Button>
      ))}
    </div>
  );
}
