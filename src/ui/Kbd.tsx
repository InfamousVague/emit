import "./Kbd.css";

interface KbdProps {
  children: React.ReactNode;
  className?: string;
}

export function Kbd({ children, className = "" }: KbdProps) {
  return <kbd className={`emit-kbd ${className}`}>{children}</kbd>;
}
