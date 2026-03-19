import { GradientText } from "./GradientText";

interface HighlightedTextProps {
  text: string;
  indices: number[];
  className?: string;
}

/**
 * Renders text with matched character positions highlighted using GradientText.
 * Consecutive matched chars are grouped into a single gradient span.
 */
export function HighlightedText({
  text,
  indices,
  className = "",
}: HighlightedTextProps) {
  if (indices.length === 0) {
    return <span className={className}>{text}</span>;
  }

  const matchSet = new Set(indices);
  const segments: { text: string; matched: boolean }[] = [];
  let current = "";
  let currentMatched = false;

  for (let i = 0; i < text.length; i++) {
    const isMatch = matchSet.has(i);
    if (i === 0) {
      currentMatched = isMatch;
      current = text[i];
    } else if (isMatch === currentMatched) {
      current += text[i];
    } else {
      segments.push({ text: current, matched: currentMatched });
      current = text[i];
      currentMatched = isMatch;
    }
  }
  if (current) {
    segments.push({ text: current, matched: currentMatched });
  }

  return (
    <span className={className}>
      {segments.map((seg, i) =>
        seg.matched ? (
          <GradientText key={i}>{seg.text}</GradientText>
        ) : (
          <span key={i}>{seg.text}</span>
        )
      )}
    </span>
  );
}
