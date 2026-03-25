import { memo } from "react";
import { Text } from "@base/primitives/text/Text";

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
      className={className}
      style={{ textTransform: "uppercase", letterSpacing: "0.05em" }}
    >
      {children}
    </Text>
  );
});
