import { useCallback, useEffect, useState } from "react";
import type { NotionPage } from "../../lib/types";
import {
  getExtensionSettings,
  notionGetDatabases,
  notionQueryDatabase,
  saveExtensionSettings,
} from "../../lib/tauri";
import { Select } from "../../ui";
import "./NotionView.css";

interface NotionViewProps {
  filter: string;
  onBack: () => void;
  onTrailingChange: (node: React.ReactNode) => void;
}

interface SavedFilter {
  name: string;
  status: string;
  assignee: string;
}

export function NotionView({ filter, onBack, onTrailingChange }: NotionViewProps) {
  const [pages, setPages] = useState<NotionPage[]>([]);
  const [databases, setDatabases] = useState<{ id: string; title: string }[]>(
    [],
  );
  const [selectedDb, setSelectedDb] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [assigneeFilter, setAssigneeFilter] = useState("");
  const [savedFilters, setSavedFilters] = useState<SavedFilter[]>([]);
  const [activeFilter, setActiveFilter] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [configured, setConfigured] = useState(true);

  // Load databases and saved filters
  useEffect(() => {
    notionGetDatabases()
      .then((dbs) => {
        setDatabases(dbs);
        if (dbs.length > 0) setSelectedDb(dbs[0].id);
      })
      .catch(() => setConfigured(false));

    getExtensionSettings("notion").then((s) => {
      const settings = s as Record<string, unknown>;
      const filters = (settings.saved_filters as SavedFilter[]) ?? [];
      setSavedFilters(filters);
      if (settings.default_database_id) {
        setSelectedDb(settings.default_database_id as string);
      }
    });
  }, []);

  // Query database when filters change
  useEffect(() => {
    if (!selectedDb) return;
    notionQueryDatabase(selectedDb, {
      status: statusFilter,
      assignee: assigneeFilter,
    })
      .then(setPages)
      .catch(() => setPages([]));
  }, [selectedDb, statusFilter, assigneeFilter]);

  // Filter results by search text
  const filtered = filter
    ? pages.filter(
        (p) =>
          p.title.toLowerCase().includes(filter.toLowerCase()) ||
          p.status.toLowerCase().includes(filter.toLowerCase()) ||
          p.assignee.toLowerCase().includes(filter.toLowerCase()),
      )
    : pages;

  // Set up trailing element with database selector
  useEffect(() => {
    if (databases.length > 0) {
      onTrailingChange(
        <Select
          variant="pill"
          value={selectedDb}
          options={databases.map((db) => ({
            value: db.id,
            label: db.title,
          }))}
          onChange={(v) => setSelectedDb(String(v))}
        />,
      );
    }
    return () => onTrailingChange(null);
  }, [databases, selectedDb, onTrailingChange]);

  // Keyboard navigation
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && filtered[selectedIndex]) {
        e.preventDefault();
        window.open(filtered[selectedIndex].url, "_blank");
      } else if (e.key === "Escape") {
        e.preventDefault();
        onBack();
      }
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [filtered, selectedIndex, onBack]);

  const applySavedFilter = (f: SavedFilter) => {
    setStatusFilter(f.status);
    setAssigneeFilter(f.assignee);
    setActiveFilter(f.name);
  };

  const saveCurrentFilter = async () => {
    const name = `Filter ${savedFilters.length + 1}`;
    const newFilter: SavedFilter = {
      name,
      status: statusFilter,
      assignee: assigneeFilter,
    };
    const updated = [...savedFilters, newFilter];
    setSavedFilters(updated);

    const settings = (await getExtensionSettings("notion")) as Record<
      string,
      unknown
    >;
    await saveExtensionSettings("notion", {
      ...settings,
      saved_filters: updated,
    });
  };

  if (!configured) {
    return (
      <div className="notion-view">
        <div className="notion-empty">
          Notion is not configured. Go to Extensions to add your API key.
        </div>
      </div>
    );
  }

  return (
    <div className="notion-view">
      <div className="notion-filters">
        <input
          className="notion-filter-select"
          placeholder="Status..."
          value={statusFilter}
          onChange={(e) => {
            setStatusFilter(e.target.value);
            setActiveFilter(null);
          }}
        />
        <input
          className="notion-filter-select"
          placeholder="Assignee..."
          value={assigneeFilter}
          onChange={(e) => {
            setAssigneeFilter(e.target.value);
            setActiveFilter(null);
          }}
        />
      </div>

      {savedFilters.length > 0 && (
        <div className="notion-saved-bar">
          {savedFilters.map((f) => (
            <button
              key={f.name}
              className={`notion-saved-chip ${activeFilter === f.name ? "active" : ""}`}
              onClick={() => applySavedFilter(f)}
            >
              {f.name}
            </button>
          ))}
          <button className="notion-save-filter-btn" onClick={saveCurrentFilter}>
            + Save
          </button>
        </div>
      )}

      {savedFilters.length === 0 &&
        (statusFilter || assigneeFilter) && (
          <div className="notion-saved-bar">
            <button
              className="notion-save-filter-btn"
              onClick={saveCurrentFilter}
            >
              + Save current filter
            </button>
          </div>
        )}

      <div className="notion-results">
        {filtered.length === 0 ? (
          <div className="notion-empty">
            {pages.length === 0
              ? "Loading Notion pages..."
              : "No results match your filters"}
          </div>
        ) : (
          filtered.map((page, i) => (
            <div
              key={page.id}
              className={`notion-item ${i === selectedIndex ? "selected" : ""}`}
              onClick={() => window.open(page.url, "_blank")}
            >
              <div className="notion-item-info">
                <div className="notion-item-title">{page.title}</div>
                {page.assignee && (
                  <div className="notion-item-meta">{page.assignee}</div>
                )}
              </div>
              {page.status && (
                <span className="notion-status-badge">{page.status}</span>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
