import "./TabNav.css";

export interface Tab {
  id: string;
  label: string;
}

interface Props {
  tabs: Tab[];
  activeId: string;
  onChange: (id: string) => void;
}

export function TabNav({ tabs, activeId, onChange }: Props) {
  return (
    <div className="emit-tab-nav">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          className={`emit-tab-nav-item ${activeId === tab.id ? "active" : ""}`}
          onClick={() => onChange(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
