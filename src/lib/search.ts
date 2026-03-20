/** Find all character indices where `needle` matches in `haystack` (case-insensitive). */
export function substringMatchIndices(haystack: string, needle: string): number[] {
  if (!needle) return [];
  const lower = haystack.toLowerCase();
  const target = needle.toLowerCase();
  const indices: number[] = [];
  let start = 0;
  let pos: number;
  while ((pos = lower.indexOf(target, start)) !== -1) {
    for (let i = pos; i < pos + target.length; i++) indices.push(i);
    start = pos + 1;
  }
  return indices;
}
