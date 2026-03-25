import { memo } from "react";
import { Badge as BaseBadge } from "@base/primitives/badge/Badge";

type BadgeVariant = "default" | "success" | "warning" | "error";

const COLOR_MAP = {
  default: "neutral",
  success: "success",
  warning: "warning",
  error: "error",
} as const;

interface BadgeProps {
  variant?: BadgeVariant;
  className?: string;
  children: React.ReactNode;
}

export const Badge = memo(function Badge({
  variant = "default",
  className = "",
  children,
}: BadgeProps) {
  return (
    <BaseBadge color={COLOR_MAP[variant]} className={className}>
      {children}
    </BaseBadge>
  );
});
