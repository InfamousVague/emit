import { Tabs } from "@base/primitives/tabs/Tabs";
import type { Tab as BaseTab } from "@base/primitives/tabs/Tabs";

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
  const baseTabs: BaseTab[] = tabs.map((t) => ({ value: t.id, label: t.label }));
  return <Tabs tabs={baseTabs} value={activeId} onChange={onChange} size="sm" />;
}
