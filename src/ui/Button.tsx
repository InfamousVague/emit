import { Button as BaseButton } from "@base/primitives/button/Button";
import type { ButtonProps as BaseButtonProps } from "@base/primitives/button/Button";

type EmitButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends Omit<BaseButtonProps, "variant" | "intent"> {
  variant?: EmitButtonVariant;
}

export function Button({ variant = "secondary", ...rest }: ButtonProps) {
  if (variant === "danger") {
    return <BaseButton variant="secondary" intent="error" {...rest} />;
  }
  return <BaseButton variant={variant} {...rest} />;
}
