import "./Badge.css";

type BadgeVariant = "default" | "success" | "warning" | "error";

interface BadgeProps {
  variant?: BadgeVariant;
  className?: string;
  children: React.ReactNode;
}

export function Badge({
  variant = "default",
  className = "",
  children,
}: BadgeProps) {
  return (
    <span className={`emit-badge emit-badge--${variant} ${className}`}>
      {children}
    </span>
  );
}
