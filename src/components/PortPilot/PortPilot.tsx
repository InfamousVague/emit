import { useCallback, useEffect, useMemo, useState } from "react";
import { portListListeners, portKillProcess } from "../../lib/tauri";
import type { PortListener } from "../../lib/types";
import { Badge, Button, Text } from "../../ui";
import "./PortPilot.css";

interface Props {
  filter: string;
  onBack: () => void;
  onTrailingChange: (node: React.ReactNode) => void;
}

/** Well-known ports mapped to friendly service names. */
const KNOWN_PORTS: Record<number, string> = {
  80: "HTTP",
  443: "HTTPS",
  3000: "Dev Server",
  3001: "Dev Server",
  4000: "Dev Server",
  4200: "Angular",
  5000: "Flask / Vite",
  5173: "Vite",
  5174: "Vite HMR",
  5432: "PostgreSQL",
  5500: "Live Server",
  6379: "Redis",
  8000: "Django / API",
  8080: "HTTP Proxy",
  8081: "Metro Bundler",
  8443: "HTTPS Alt",
  8888: "Jupyter",
  9000: "PHP-FPM",
  9090: "Prometheus",
  9229: "Node Debug",
  27017: "MongoDB",
};

/** Map process names to friendly category labels. */
function processCategory(name: string): string {
  const lower = name.toLowerCase();
  if (lower.includes("node") || lower.includes("deno") || lower.includes("bun")) return "JavaScript";
  if (lower.includes("python") || lower.includes("uvicorn") || lower.includes("gunicorn")) return "Python";
  if (lower.includes("ruby") || lower.includes("puma")) return "Ruby";
  if (lower.includes("java") || lower.includes("gradle")) return "Java";
  if (lower.includes("go") || lower.includes("air")) return "Go";
  if (lower.includes("postgres")) return "Database";
  if (lower.includes("mysql") || lower.includes("mariadbd")) return "Database";
  if (lower.includes("redis") || lower.includes("memcache")) return "Cache";
  if (lower.includes("mongo")) return "Database";
  if (lower.includes("nginx") || lower.includes("apache") || lower.includes("httpd")) return "Web Server";
  if (lower.includes("docker") || lower.includes("containerd")) return "Container";
  return "System";
}

/** Truncate a command string to a max length, keeping the end. */
function truncateCommand(cmd: string, max: number): string {
  if (cmd.length <= max) return cmd;
  return "…" + cmd.slice(cmd.length - max + 1);
}

export function PortPilot({ filter, onTrailingChange }: Props) {
  const [listeners, setListeners] = useState<PortListener[]>([]);
  const [loading, setLoading] = useState(true);
  const [killingPid, setKillingPid] = useState<number | null>(null);
  const [killResult, setKillResult] = useState<{ pid: number; msg: string; ok: boolean } | null>(null);
  const [sortBy, setSortBy] = useState<"port" | "name" | "pid">("port");

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await portListListeners();
      setListeners(data);
    } catch {
      setListeners([]);
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

  // Clear kill result after 3s
  useEffect(() => {
    if (!killResult) return;
    const t = setTimeout(() => setKillResult(null), 3000);
    return () => clearTimeout(t);
  }, [killResult]);

  const handleKill = useCallback(
    async (pid: number) => {
      setKillingPid(pid);
      try {
        const msg = await portKillProcess(pid);
        setKillResult({ pid, msg, ok: true });
        // Re-scan after kill
        setTimeout(refresh, 500);
      } catch (e) {
        setKillResult({ pid, msg: String(e), ok: false });
      } finally {
        setKillingPid(null);
      }
    },
    [refresh],
  );

  // Filter and sort
  const filtered = useMemo(() => {
    let items = listeners;
    if (filter) {
      const q = filter.toLowerCase();
      items = items.filter(
        (l) =>
          l.process_name.toLowerCase().includes(q) ||
          l.command.toLowerCase().includes(q) ||
          String(l.port).includes(q) ||
          String(l.pid).includes(q) ||
          (KNOWN_PORTS[l.port] ?? "").toLowerCase().includes(q),
      );
    }
    return [...items].sort((a, b) => {
      if (sortBy === "port") return a.port - b.port;
      if (sortBy === "name") return a.process_name.localeCompare(b.process_name);
      return a.pid - b.pid;
    });
  }, [listeners, filter, sortBy]);

  // Deduplicate by port for summary stats
  const uniquePorts = useMemo(
    () => new Set(listeners.map((l) => l.port)).size,
    [listeners],
  );

  const categories = useMemo(() => {
    const cats = new Map<string, number>();
    for (const l of listeners) {
      const cat = processCategory(l.process_name);
      cats.set(cat, (cats.get(cat) ?? 0) + 1);
    }
    return cats;
  }, [listeners]);

  if (loading) {
    return (
      <div className="port-pilot">
        <div className="port-pilot__loading">
          <span className="search-spinner" />
          <Text size="sm" color="secondary">Scanning ports…</Text>
        </div>
      </div>
    );
  }

  return (
    <div className="port-pilot">
      {/* Summary bar */}
      <div className="port-pilot__summary">
        <div className="port-pilot__stats">
          <Text size="sm" color="secondary">
            <strong>{uniquePorts}</strong> ports · <strong>{listeners.length}</strong> listeners
          </Text>
          <div className="port-pilot__categories">
            {Array.from(categories.entries()).map(([cat, count]) => (
              <Badge key={cat} variant={cat === "Database" ? "warning" : cat === "Container" ? "error" : "default"}>
                {cat} ({count})
              </Badge>
            ))}
          </div>
        </div>
        <div className="port-pilot__sort">
          <Text size="xs" color="secondary">Sort:</Text>
          {(["port", "name", "pid"] as const).map((s) => (
            <Button
              key={s}
              size="sm"
              variant="ghost"
              className={sortBy === s ? "port-pilot__sort-btn--active" : ""}
              onClick={() => setSortBy(s)}
            >
              {s.charAt(0).toUpperCase() + s.slice(1)}
            </Button>
          ))}
        </div>
      </div>

      {/* Kill result toast */}
      {killResult && (
        <div className={`port-pilot__toast port-pilot__toast--${killResult.ok ? "success" : "error"}`}>
          <Text size="xs">{killResult.msg}</Text>
        </div>
      )}

      {/* Port list */}
      <div className="port-pilot__list">
        {filtered.length === 0 ? (
          <div className="port-pilot__empty">
            <Text size="sm" color="secondary">
              {filter ? "No matching ports" : "No listening ports found"}
            </Text>
          </div>
        ) : (
          filtered.map((l, i) => (
            <div key={`${l.pid}-${l.port}-${i}`} className="port-pilot__row">
              <div className="port-pilot__port">
                <Text size="base" weight="semibold" tabular>
                  :{l.port}
                </Text>
                {KNOWN_PORTS[l.port] && (
                  <Text size="2xs" color="secondary">
                    {KNOWN_PORTS[l.port]}
                  </Text>
                )}
              </div>
              <div className="port-pilot__info">
                <div className="port-pilot__process">
                  <Text size="sm" weight="medium">{l.process_name}</Text>
                  <Badge variant="default">{processCategory(l.process_name)}</Badge>
                </div>
                <Text size="xs" color="secondary" className="port-pilot__command">
                  {truncateCommand(l.command, 80)}
                </Text>
              </div>
              <div className="port-pilot__meta">
                <Text size="2xs" color="secondary" tabular>PID {l.pid}</Text>
                <Text size="2xs" color="secondary">{l.user}</Text>
              </div>
              <div className="port-pilot__actions">
                <Button
                  size="sm"
                  variant="danger"
                  onClick={() => handleKill(l.pid)}
                  disabled={killingPid === l.pid}
                >
                  {killingPid === l.pid ? "…" : "Kill"}
                </Button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
