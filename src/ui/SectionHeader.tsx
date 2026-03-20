import { memo } from "react";
import { Text } from "./Text";

interface SectionHeaderProps {
  children: React.ReactNode;
  className?: string;
}

export const SectionHeader = memo(function SectionHeader({ children, className }: SectionHeaderProps) {
  return (
    <Text
      as="div"
      size="xs"
      weight="semibold"
      color="secondary"
      uppercase
      className={className}
    >
      {children}
    </Text>
  );
});
