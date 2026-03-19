import type { CommandEntry } from "./types";

/** Group command entries by their category, preserving insertion order. */
export function groupByCategory(
  entries: CommandEntry[]
): [string, CommandEntry[]][] {
  const map: Record<string, CommandEntry[]> = {};
  for (const cmd of entries) {
    if (!map[cmd.category]) map[cmd.category] = [];
    map[cmd.category].push(cmd);
  }
  return Object.entries(map);
}
