import { HighlightMatch } from "@base/primitives/animation/HighlightMatch";

interface HighlightedTextProps {
  text: string;
  indices: number[];
  className?: string;
}

export function HighlightedText({ text, indices, className }: HighlightedTextProps) {
  return (
    <HighlightMatch
      text={text}
      indices={indices}
      variant="gradient"
      className={className}
    />
  );
}
