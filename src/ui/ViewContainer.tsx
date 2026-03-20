import "./ViewContainer.css";

interface ViewContainerProps {
  children: React.ReactNode;
  className?: string;
}

export function ViewContainer({ children, className = "" }: ViewContainerProps) {
  return (
    <div className={`emit-view ${className}`}>
      {children}
    </div>
  );
}

interface ViewBodyProps {
  children: React.ReactNode;
  className?: string;
}

function ViewBody({ children, className = "" }: ViewBodyProps) {
  return (
    <div className={`emit-view__body ${className}`}>
      {children}
    </div>
  );
}

ViewContainer.Body = ViewBody;
