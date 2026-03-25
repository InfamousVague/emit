import { forwardRef } from "react";
import { Input as BaseInput } from "@base/primitives/input/Input";
import type { InputProps as BaseInputProps } from "@base/primitives/input/Input";

type InputVariant = "default" | "mono";
type InputSize = "sm" | "md";

interface InputProps extends Omit<BaseInputProps, "size" | "variant" | "shape"> {
  variant?: InputVariant;
  inputSize?: InputSize;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { variant = "default", inputSize = "md", className = "", style, ...props },
  ref,
) {
  // Base Input wraps in a div, but we need forwardRef for SearchInput.
  // Use a bare <input> with Base CSS classes to preserve ref semantics.
  const classes = [
    "input",
    className,
  ].filter(Boolean).join(" ");

  const wrapperClasses = [
    "input-wrapper",
    `input-wrapper--${inputSize}`,
    "input-wrapper--outline",
    variant === "mono" ? "text--mono" : "",
  ].filter(Boolean).join(" ");

  return (
    <div className={wrapperClasses} style={style}>
      <input ref={ref} className={classes} {...props} />
    </div>
  );
});
