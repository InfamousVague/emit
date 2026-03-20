import { memo } from "react";
import "./Text.css";

type TextSize = "3xs" | "2xs" | "xs" | "sm" | "base" | "md" | "lg";
type TextWeight = "normal" | "medium" | "semibold";
type TextColor = "primary" | "secondary" | "tertiary" | "muted" | "error" | "success" | "warning";

interface TextProps {
  size?: TextSize;
  weight?: TextWeight;
  color?: TextColor;
  as?: "span" | "p" | "div" | "label";
  uppercase?: boolean;
  tabular?: boolean;
  className?: string;
  children: React.ReactNode;
  style?: React.CSSProperties;
}

export const Text = memo(function Text({
  size = "md",
  weight = "normal",
  color = "primary",
  as: Tag = "span",
  uppercase = false,
  tabular = false,
  className = "",
  children,
  style,
}: TextProps) {
  const classes = [
    "emit-text",
    `emit-text--${size}`,
    `emit-text--w-${weight}`,
    `emit-text--c-${color}`,
    uppercase && "emit-text--uppercase",
    tabular && "emit-text--tabular",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <Tag className={classes} style={style}>
      {children}
    </Tag>
  );
});
