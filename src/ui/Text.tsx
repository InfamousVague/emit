import { memo } from "react";
import { Text as BaseText } from "@base/primitives/text/Text";
import type { CSSProperties } from "react";

type TextSize = "3xs" | "2xs" | "xs" | "sm" | "base" | "md" | "lg";
type TextWeight = "normal" | "medium" | "semibold";
type TextColor = "primary" | "secondary" | "tertiary" | "muted" | "error" | "success" | "warning";

const SIZE_MAP = {
  "3xs": "xs",
  "2xs": "xs",
  xs: "xs",
  sm: "sm",
  base: "base",
  md: "base",
  lg: "lg",
} as const;

const WEIGHT_MAP = {
  normal: "regular",
  medium: "medium",
  semibold: "semibold",
} as const;

const COLOR_MAP = {
  primary: "primary",
  secondary: "secondary",
  tertiary: "tertiary",
  muted: "tertiary",
  error: "error",
  success: "success",
  warning: "warning",
} as const;

interface TextProps {
  size?: TextSize;
  weight?: TextWeight;
  color?: TextColor;
  as?: "span" | "p" | "div" | "label";
  uppercase?: boolean;
  tabular?: boolean;
  className?: string;
  children: React.ReactNode;
  style?: CSSProperties;
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
  const extraStyle: CSSProperties = {
    ...style,
    ...(uppercase ? { textTransform: "uppercase" as const, letterSpacing: "0.05em" } : {}),
    ...(tabular ? { fontVariantNumeric: "tabular-nums" } : {}),
  };

  return (
    <BaseText
      as={Tag}
      size={SIZE_MAP[size]}
      weight={WEIGHT_MAP[weight]}
      color={COLOR_MAP[color]}
      className={className}
      style={Object.keys(extraStyle).length > 0 ? extraStyle : undefined}
    >
      {children}
    </BaseText>
  );
});
