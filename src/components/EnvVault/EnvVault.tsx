import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  envVaultScan,
  envVaultGetConfig,
  envVaultSaveConfig,
  envVaultOpenDir,
  envVaultUpdateVar,
} from "../../lib/tauri";
import type { EnvFile, EnvVaultConfig } from "../../lib/types";
import { Badge, Button, Input, Text } from "../../ui";
import "./EnvVault.css";

interface Props {
  filter: string;
  onBack: () => void;
  onTrailingChange: (node: React.ReactNode) => void;
}

type ViewMode = "files" | "variables";

/** Map env labels to badge variants. */
function labelVariant(label: string): "default" | "success" | "warning" | "error" {
  switch (label) {
    case "Production":
      return "error";
    case "Staging":
      return "warning";
    case "Example":
      return "success";
    default:
      return "default";
  }
}

/** Mask a value for display. */
function maskValue(value: string): string {
  if (value.length <= 4) return "••••••••";
  return value.slice(0, 2) + "••••••" + value.slice(-2);
}

/** A variable with its source file info for the flat variables view. */
interface FlatVar {
  key: string;
  value: string;
  filePath: string;
  filename: string;
  envLabel: string;
  project: string;
  relativeDir: string;
}

export function EnvVault({ filter, onTrailingChange }: Props) {
  const [files, setFiles] = useState<EnvFile[]>([]);
  const [loading, setLoading] = useState(true);
  const [config, setConfig] = useState<EnvVaultConfig>({ scan_dirs: [] });
  const [expandedPath, setExpandedPath] = useState<string | null>(null);
  const [revealedKeys, setRevealedKeys] = useState<Set<string>>(new Set());
  const [showAddDir, setShowAddDir] = useState(false);
  const [newDir, setNewDir] = useState("");
  const [toast, setToast] = useState<{ msg: string; ok: boolean } | null>(null);
  const [sortBy, setSortBy] = useState<"path" | "project" | "vars">("project");
  const [viewMode, setViewMode] = useState<ViewMode>("files");
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const editRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [cfg, scanned] = await Promise.all([
        envVaultGetConfig(),
        envVaultScan(),
      ]);
      setConfig(cfg);
      setFiles(scanned);
    } catch {
      setFiles([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    onTrailingChange(
      <Button size="sm" variant="ghost" onClick={refresh}>
        Refresh
      </Button>,
    );
    return () => onTrailingChange(null);
  }, [onTrailingChange, refresh]);

  // Toast auto-dismiss
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 3000);
    return () => clearTimeout(t);
  }, [toast]);

  // Focus edit input when editing starts
  useEffect(() => {
    if (editingKey && editRef.current) {
      editRef.current.focus();
      editRef.current.select();
    }
  }, [editingKey]);

  const handleAddDir = useCallback(async () => {
    if (!newDir.trim()) return;
    const updated = { ...config, scan_dirs: [...config.scan_dirs, newDir.trim()] };
    try {
      await envVaultSaveConfig(updated);
      setConfig(updated);
      setNewDir("");
      setShowAddDir(false);
      await refresh();
    } catch (e) {
      setToast({ msg: String(e), ok: false });
    }
  }, [config, newDir, refresh]);

  const handleRemoveDir = useCallback(
    async (dir: string) => {
      const updated = {
        ...config,
        scan_dirs: config.scan_dirs.filter((d) => d !== dir),
      };
      try {
        await envVaultSaveConfig(updated);
        setConfig(updated);
        await refresh();
      } catch (e) {
        setToast({ msg: String(e), ok: false });
      }
    },
    [config, refresh],
  );

  const toggleExpand = useCallback((path: string) => {
    setExpandedPath((prev) => (prev === path ? null : path));
    setRevealedKeys(new Set());
  }, []);

  const toggleReveal = useCallback((compositeKey: string) => {
    setRevealedKeys((prev) => {
      const next = new Set(prev);
      if (next.has(compositeKey)) next.delete(compositeKey);
      else next.add(compositeKey);
      return next;
    });
  }, []);

  const startEdit = useCallback((compositeKey: string, currentValue: string) => {
    setEditingKey(compositeKey);
    setEditValue(currentValue);
  }, []);

  const cancelEdit = useCallback(() => {
    setEditingKey(null);
    setEditValue("");
  }, []);

  const saveEdit = useCallback(async () => {
    if (!editingKey) return;
    // compositeKey format: "filePath:varKey"
    const sepIdx = editingKey.indexOf(":");
    const filePath = editingKey.substring(0, sepIdx);
    const varKey = editingKey.substring(sepIdx + 1);

    try {
      await envVaultUpdateVar(filePath, varKey, editValue);
      setToast({ msg: `Updated ${varKey}`, ok: true });
      setEditingKey(null);
      setEditValue("");
      await refresh();
    } catch (e) {
      setToast({ msg: String(e), ok: false });
    }
  }, [editingKey, editValue, refresh]);

  // Filter and sort files
  const filtered = useMemo(() => {
    let items = files;
    if (filter) {
      const q = filter.toLowerCase();
      items = items.filter(
        (f) =>
          f.project.toLowerCase().includes(q) ||
          f.filename.toLowerCase().includes(q) ||
          f.relative_dir.toLowerCase().includes(q) ||
          f.env_label.toLowerCase().includes(q) ||
          f.variables.some((v) => v.key.toLowerCase().includes(q)),
      );
    }
    return [...items].sort((a, b) => {
      if (sortBy === "project") return a.project.localeCompare(b.project) || a.filename.localeCompare(b.filename);
      if (sortBy === "vars") return b.var_count - a.var_count;
      return a.file_path.localeCompare(b.file_path);
    });
  }, [files, filter, sortBy]);

  // Build flat variable list grouped by relative_dir for the Variables view
  const groupedVars = useMemo(() => {
    const allVars: FlatVar[] = files.flatMap((f) =>
      f.variables.map((v) => ({
        key: v.key,
        value: v.value,
        filePath: f.file_path,
        filename: f.filename,
        envLabel: f.env_label,
        project: f.project,
        relativeDir: f.relative_dir,
      })),
    );

    // Filter
    const q = filter?.toLowerCase() ?? "";
    const filteredVars = q
      ? allVars.filter(
          (v) =>
            v.key.toLowerCase().includes(q) ||
            v.value.toLowerCase().includes(q) ||
            v.project.toLowerCase().includes(q) ||
            v.relativeDir.toLowerCase().includes(q),
        )
      : allVars;

    // Group by relative_dir (project path)
    const groups = new Map<string, FlatVar[]>();
    for (const v of filteredVars) {
      const groupKey = v.relativeDir || v.project;
      if (!groups.has(groupKey)) groups.set(groupKey, []);
      groups.get(groupKey)!.push(v);
    }

    // Sort groups by name, vars within each group by key
    const sorted = Array.from(groups.entries()).sort(([a], [b]) => a.localeCompare(b));
    for (const [, vars] of sorted) {
      vars.sort((a, b) => a.key.localeCompare(b.key));
    }

    return sorted;
  }, [files, filter]);

  // Stats
  const uniqueProjects = useMemo(
    () => new Set(files.map((f) => f.project)).size,
    [files],
  );
  const totalVars = useMemo(
    () => files.reduce((sum, f) => sum + f.var_count, 0),
    [files],
  );
  const envTypes = useMemo(() => {
    const types = new Map<string, number>();
    for (const f of files) {
      types.set(f.env_label, (types.get(f.env_label) ?? 0) + 1);
    }
    return types;
  }, [files]);

  if (loading) {
    return (
      <div className="env-vault">
        <div className="env-vault__loading">
          <span className="search-spinner" />
          <Text size="sm" color="secondary">Scanning for .env files…</Text>
        </div>
      </div>
    );
  }

  return (
    <div className="env-vault">
      {/* Toast */}
      {toast && (
        <div className={`env-vault__toast env-vault__toast--${toast.ok ? "success" : "error"}`}>
          <Text size="xs">{toast.msg}</Text>
        </div>
      )}

      {/* Summary bar */}
      <div className="env-vault__summary">
        <div className="env-vault__stats">
          {config.scan_dirs.length > 0 ? (
            <>
              <Text size="sm" color="secondary">
                <strong>{files.length}</strong> files · <strong>{uniqueProjects}</strong> projects · <strong>{totalVars}</strong> vars
              </Text>
              <div className="env-vault__categories">
                {Array.from(envTypes.entries()).map(([label, count]) => (
                  <Badge key={label} variant={labelVariant(label)}>
                    {label} ({count})
                  </Badge>
                ))}
              </div>
            </>
          ) : (
            <Text size="sm" color="secondary">Add a directory to scan</Text>
          )}
        </div>
        <div className="env-vault__sort">
          {config.scan_dirs.length > 0 && files.length > 0 && (
            <div className="env-vault__view-toggle">
              <Button
                size="sm"
                variant="ghost"
                className={viewMode === "files" ? "env-vault__sort-btn--active" : ""}
                onClick={() => setViewMode("files")}
              >
                Files
              </Button>
              <Button
                size="sm"
                variant="ghost"
                className={viewMode === "variables" ? "env-vault__sort-btn--active" : ""}
                onClick={() => setViewMode("variables")}
              >
                Variables
              </Button>
            </div>
          )}
          <Button size="sm" variant="ghost" onClick={() => setShowAddDir(!showAddDir)}>
            {showAddDir ? "Cancel" : "+ Dir"}
          </Button>
        </div>
      </div>

      {/* Add directory form */}
      {showAddDir && (
        <div className="env-vault__add-form">
          {config.scan_dirs.map((dir) => (
            <div key={dir} className="env-vault__dir-row">
              <Text size="xs">{dir}</Text>
              <Button
                size="sm"
                variant="ghost"
                aria-label="Remove directory"
                onClick={() => handleRemoveDir(dir)}
              >
                ✕
              </Button>
            </div>
          ))}
          <div className="env-vault__add-row">
            <Input
              variant="mono"
              inputSize="sm"
              placeholder="/Users/you/Development"
              value={newDir}
              onChange={(e) => setNewDir(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAddDir()}
              autoFocus
            />
            <Button size="sm" variant="primary" onClick={handleAddDir}>
              Add
            </Button>
          </div>
        </div>
      )}

      {/* ── Variables view ── */}
      {viewMode === "variables" && (
        <div className="env-vault__list">
          {groupedVars.length === 0 ? (
            <div className="env-vault__empty">
              <Text size="sm" color="secondary">
                {filter ? "No matching variables" : "No variables found"}
              </Text>
            </div>
          ) : (
            groupedVars.map(([groupKey, vars]) => (
              <div key={groupKey} className="env-vault__group">
                <div className="env-vault__group-header">
                  <Text size="xs" weight="semibold" color="secondary">
                    {groupKey}
                  </Text>
                  <div className="env-vault__group-badges">
                    {Array.from(new Set(vars.map((v) => v.envLabel))).map((label) => (
                      <Badge key={label} variant={labelVariant(label)}>
                        {label}
                      </Badge>
                    ))}
                  </div>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      const dir = vars[0].filePath.substring(0, vars[0].filePath.lastIndexOf("/"));
                      envVaultOpenDir(dir);
                    }}
                  >
                    Open
                  </Button>
                </div>
                {vars.map((v) => {
                  const compositeKey = `${v.filePath}:${v.key}`;
                  const isRevealed = revealedKeys.has(compositeKey);
                  const isEditing = editingKey === compositeKey;

                  return (
                    <div key={compositeKey} className="env-vault__var-row">
                      <Text size="xs" weight="medium" className="env-vault__var-key">
                        {v.key}
                      </Text>
                      {isEditing ? (
                        <div className="env-vault__var-edit">
                          <Input
                            ref={editRef}
                            variant="mono"
                            inputSize="sm"
                            value={editValue}
                            onChange={(e) => setEditValue(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") saveEdit();
                              if (e.key === "Escape") cancelEdit();
                            }}
                            onBlur={cancelEdit}
                          />
                        </div>
                      ) : (
                        <div
                          className="env-vault__var-value"
                          onClick={() => toggleReveal(compositeKey)}
                        >
                          <Text size="xs" color="secondary">
                            {isRevealed ? v.value : maskValue(v.value)}
                          </Text>
                        </div>
                      )}
                      <div className="env-vault__var-source">
                        <Text size="2xs" color="secondary">{v.filename}</Text>
                      </div>
                      {!isEditing && (
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label="Edit value"
                          onClick={() => startEdit(compositeKey, v.value)}
                        >
                          Edit
                        </Button>
                      )}
                    </div>
                  );
                })}
              </div>
            ))
          )}
        </div>
      )}

      {/* ── Files view ── */}
      {viewMode === "files" && (
        <div className="env-vault__list">
          {filtered.length === 0 ? (
            <div className="env-vault__empty">
              <Text size="sm" color="secondary">
                {filter ? "No matching .env files" : config.scan_dirs.length === 0 ? "Add a scan directory to get started" : "No .env files found"}
              </Text>
            </div>
          ) : (
            filtered.map((f) => (
              <div key={f.file_path} className="env-vault__file">
                <div
                  className="env-vault__row"
                  onClick={() => toggleExpand(f.file_path)}
                >
                  <div className="env-vault__file-name">
                    <Text size="base" weight="semibold">
                      {f.filename}
                    </Text>
                    <Badge variant={labelVariant(f.env_label)}>
                      {f.env_label}
                    </Badge>
                  </div>
                  <div className="env-vault__file-info">
                    <Text size="sm" weight="medium">{f.project}</Text>
                    <Text size="xs" color="secondary" className="env-vault__path">
                      {f.relative_dir}
                    </Text>
                  </div>
                  <div className="env-vault__file-meta">
                    <Text size="2xs" color="secondary" tabular>
                      {f.var_count} vars
                    </Text>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={(e) => {
                        e.stopPropagation();
                        const dir = f.file_path.substring(0, f.file_path.lastIndexOf("/"));
                        envVaultOpenDir(dir);
                      }}
                    >
                      Open
                    </Button>
                  </div>
                </div>

                {/* Expanded variables */}
                {expandedPath === f.file_path && (
                  <div className="env-vault__vars">
                    {f.variables.map((v) => {
                      const compositeKey = `${f.file_path}:${v.key}`;
                      const isRevealed = revealedKeys.has(compositeKey);
                      const isEditing = editingKey === compositeKey;

                      return (
                        <div key={v.key} className="env-vault__var-row">
                          <Text size="xs" weight="medium" className="env-vault__var-key">
                            {v.key}
                          </Text>
                          {isEditing ? (
                            <div className="env-vault__var-edit">
                              <Input
                                ref={editRef}
                                variant="mono"
                                inputSize="sm"
                                value={editValue}
                                onChange={(e) => setEditValue(e.target.value)}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") saveEdit();
                                  if (e.key === "Escape") cancelEdit();
                                }}
                                onBlur={cancelEdit}
                              />
                            </div>
                          ) : (
                            <div
                              className="env-vault__var-value"
                              onClick={() => toggleReveal(compositeKey)}
                            >
                              <Text size="xs" color="secondary">
                                {isRevealed ? v.value : maskValue(v.value)}
                              </Text>
                            </div>
                          )}
                          {!isEditing && (
                            <Button
                              size="sm"
                              variant="ghost"
                              aria-label="Edit value"
                              onClick={() => startEdit(compositeKey, v.value)}
                            >
                              Edit
                            </Button>
                          )}
                        </div>
                      );
                    })}
                    {f.variables.length === 0 && (
                      <Text size="xs" color="secondary" style={{ padding: "var(--space-xs)" }}>
                        Empty file
                      </Text>
                    )}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
