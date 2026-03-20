import { memo, useEffect, useRef } from "react";
import "./ListItem.css";

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

  return (
    <div
      ref={ref}
      className={`emit-list-item ${selected ? "emit-list-item--selected" : ""} ${className}`}
      onClick={onClick}
    >
      {icon && <div className="emit-list-item__icon">{icon}</div>}
      <div className="emit-list-item__content">
        <div className="emit-list-item__title">{title}</div>
        {description && (
          <div className="emit-list-item__desc">{description}</div>
        )}
      </div>
      {trailing && <div className="emit-list-item__trailing">{trailing}</div>}
    </div>
  );
});
