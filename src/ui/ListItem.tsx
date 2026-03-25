import { memo, useEffect, useRef } from "react";

interface ListItemProps {
  icon?: React.ReactNode;
  title: React.ReactNode;
  description?: React.ReactNode;
  trailing?: React.ReactNode;
  selected?: boolean;
  onClick?: () => void;
  className?: string;
}

export const ListItem = memo(function ListItem({
  icon,
  title,
  description,
  trailing,
  selected = false,
  onClick,
  className = "",
}: ListItemProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (selected) {
      ref.current?.scrollIntoView({ block: "nearest" });
    }
  }, [selected]);

  const classes = [
    "list-item",
    selected ? "list-item--active" : "",
    className,
  ].filter(Boolean).join(" ");

  return (
    <div
      ref={ref}
      className={classes}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      {icon && <span className="list-item__icon">{icon}</span>}
      <span className="list-item__content">
        <span className="list-item__label">{title}</span>
        {description && (
          <span className="list-item__description">{description}</span>
        )}
      </span>
      {trailing && <span className="list-item__trailing">{trailing}</span>}
    </div>
  );
});
