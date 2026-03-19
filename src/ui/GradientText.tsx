import { useMemo } from "react";
import "./GradientText.css";

const DEFAULT_COLORS = ["#8B5CF6", "#EC4899", "#3B82F6", "#8B5CF6"];

interface GradientTextProps {
  children: React.ReactNode;
  colors?: string[];
  animated?: boolean;
  speed?: number;
  className?: string;
}

export function GradientText({
  children,
  colors = DEFAULT_COLORS,
  animated = true,
  speed = 3000,
  className = "",
}: GradientTextProps) {
  const style = useMemo(
    () => ({
      backgroundImage: `linear-gradient(90deg, ${colors.join(", ")})`,
      backgroundSize: animated ? "300% 300%" : undefined,
      animationDuration: animated ? `${speed}ms` : undefined,
    }),
    [colors, animated, speed]
  );

  return (
    <span
      className={`emit-gradient-text ${animated ? "animated" : ""} ${className}`}
      style={style}
    >
      {children}
    </span>
  );
}
