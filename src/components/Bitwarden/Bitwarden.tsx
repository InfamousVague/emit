import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  bwIsUnlocked,
  bwUnlock,
  bwLock,
  bwSearch,
  bwGetPassword,
  bwGetUsername,
  bwGetTotp,
  bwCopyToClipboard,
  bwStatus,
  bwGetLockTimeout,
  bwSetLockTimeout,
} from "../../lib/tauri";
import type { VaultItem } from "../../lib/types";
import { Badge, Button, Input, Text } from "../../ui";
import "./Bitwarden.css";

interface Props {
  filter: string;
  onBack: () => void;
  onTrailingChange: (node: React.ReactNode) => void;
}

type CopyField = "password" | "username" | "totp";

const TIMEOUT_OPTIONS = [
  { label: "15 minutes", value: 900 },
  { label: "1 hour", value: 3600 },
  { label: "4 hours", value: 14400 },
  { label: "8 hours", value: 28800 },
  { label: "Never", value: 0 },
];

export function Bitwarden({ filter, onTrailingChange }: Props) {
  const [status, setStatus] = useState<string>("checking");
  const [unlocked, setUnlocked] = useState(false);
  const [masterPassword, setMasterPassword] = useState("");
  const [unlocking, setUnlocking] = useState(false);
  const [unlockError, setUnlockError] = useState("");

  const [items, setItems] = useState<VaultItem[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [copied, setCopied] = useState<{ id: string; field: CopyField } | null>(null);
  const [error, setError] = useState("");
  const [lockTimeout, setLockTimeout] = useState(14400);

  const passwordRef = useRef<HTMLInputElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const copyTimeout = useRef<ReturnType<typeof setTimeout>>(undefined);

  // ── Check status on mount ───────────────────────────────────────────────
  useEffect(() => {
    (async () => {
      try {
        const s = await bwStatus();
        setStatus(s);
        if (s === "locked") {
          const isUnlocked = await bwIsUnlocked();
          if (isUnlocked) {
            setUnlocked(true);
            setStatus("unlocked");
          }
        } else if (s === "unlocked") {
          const isUnlocked = await bwIsUnlocked();
          setUnlocked(isUnlocked);
        }
        // Load current timeout
        const timeout = await bwGetLockTimeout();
        setLockTimeout(timeout);
      } catch (e) {
        setStatus("not_installed");
        setError(String(e));
      }
    })();
  }, []);

  // ── Trailing action bar ─────────────────────────────────────────────────
  useEffect(() => {
    onTrailingChange(
      unlocked ? (
        <div className="bw__trailing">
          <select
            className="bw__timeout-select"
            value={lockTimeout}
            onChange={(e) => {
              const val = Number(e.target.value);
              setLockTimeout(val);
              bwSetLockTimeout(val);
            }}
          >
            {TIMEOUT_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
          <Button size="sm" variant="ghost" onClick={handleLock}>
            Lock
          </Button>
        </div>
      ) : null,
    );
    return () => onTrailingChange(null);
  }, [unlocked, lockTimeout]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Focus ───────────────────────────────────────────────────────────────
  useEffect(() => {
    if (unlocked) {
      searchRef.current?.focus();
    } else if (status === "locked") {
      passwordRef.current?.focus();
    }
  }, [unlocked, status]);

  // ── Unlock ──────────────────────────────────────────────────────────────
  const handleUnlock = useCallback(async () => {
    if (!masterPassword.trim()) return;
    setUnlocking(true);
    setUnlockError("");
    try {
      await bwUnlock(masterPassword);
      setUnlocked(true);
      setStatus("unlocked");
      setMasterPassword("");
    } catch (e) {
      setUnlockError(String(e));
    } finally {
      setUnlocking(false);
    }
  }, [masterPassword]);

  // ── Lock ────────────────────────────────────────────────────────────────
  const handleLock = useCallback(async () => {
    try {
      await bwLock();
      setUnlocked(false);
      setStatus("locked");
      setItems([]);
      setSearchQuery("");
    } catch (e) {
      setError(String(e));
    }
  }, []);

  // ── Search ──────────────────────────────────────────────────────────────
  const handleSearch = useCallback(async (q: string) => {
    setSearchQuery(q);
    if (q.trim().length < 2) {
      setItems([]);
      return;
    }
    setSearching(true);
    setError("");
    try {
      const results = await bwSearch(q.trim());
      setItems(results);
    } catch (e) {
      setError(String(e));
      setItems([]);
    } finally {
      setSearching(false);
    }
  }, []);

  // Debounced search
  const searchTimeout = useRef<ReturnType<typeof setTimeout>>(undefined);
  const onSearchChange = useCallback(
    (val: string) => {
      setSearchQuery(val);
      clearTimeout(searchTimeout.current);
      searchTimeout.current = setTimeout(() => handleSearch(val), 300);
    },
    [handleSearch],
  );

  // ── Copy (instant from cache) ───────────────────────────────────────────
  const handleCopy = useCallback(
    async (item: VaultItem, field: CopyField) => {
      try {
        let value: string;
        switch (field) {
          case "password":
            value = await bwGetPassword(item.id);
            break;
          case "username":
            value = await bwGetUsername(item.id);
            break;
          case "totp":
            value = await bwGetTotp(item.id);
            break;
        }
        await bwCopyToClipboard(value);
        setCopied({ id: item.id, field });
        clearTimeout(copyTimeout.current);
        copyTimeout.current = setTimeout(() => setCopied(null), 2000);
      } catch (e) {
        setError(String(e));
      }
    },
    [],
  );

  // ── Filter by external filter prop ──────────────────────────────────────
  const filtered = useMemo(() => {
    if (!filter) return items;
    const f = filter.toLowerCase();
    return items.filter(
      (item) =>
        item.name.toLowerCase().includes(f) ||
        item.username.toLowerCase().includes(f) ||
        item.uri.toLowerCase().includes(f),
    );
  }, [items, filter]);

  // ── Group by folder ─────────────────────────────────────────────────────
  const grouped = useMemo(() => {
    const map = new Map<string, VaultItem[]>();
    for (const item of filtered) {
      const folder = item.folder || "No Folder";
      if (!map.has(folder)) map.set(folder, []);
      map.get(folder)!.push(item);
    }
    return [...map.entries()].sort(([a], [b]) => {
      if (a === "No Folder") return 1;
      if (b === "No Folder") return -1;
      return a.localeCompare(b);
    });
  }, [filtered]);

  // ── Not installed ───────────────────────────────────────────────────────
  if (status === "not_installed" || status === "unauthenticated") {
    return (
      <div className="bw">
        <div className="bw__empty">
          <Text size="sm" color="muted">
            {status === "unauthenticated"
              ? "Not logged in. Run 'bw login' in your terminal first."
              : "Bitwarden CLI not found. Install with: npm install -g @bitwarden/cli"}
          </Text>
        </div>
      </div>
    );
  }

  // ── Checking ────────────────────────────────────────────────────────────
  if (status === "checking") {
    return (
      <div className="bw">
        <div className="bw__loading">
          <Text size="sm" color="muted">Checking Bitwarden status…</Text>
        </div>
      </div>
    );
  }

  // ── Locked — show unlock form ───────────────────────────────────────────
  if (!unlocked) {
    return (
      <div className="bw">
        <div className="bw__unlock">
          <div className="bw__unlock-icon">🔒</div>
          <Text size="sm" color="muted">
            Enter your master password to unlock the vault
          </Text>
          <form
            className="bw__unlock-form"
            onSubmit={(e) => {
              e.preventDefault();
              handleUnlock();
            }}
          >
            <Input
              ref={passwordRef}
              type="password"
              placeholder="Master password"
              value={masterPassword}
              onChange={(e) => setMasterPassword(e.target.value)}
              autoFocus
            />
            <Button
              size="sm"
              variant="primary"
              onClick={handleUnlock}
              disabled={unlocking || !masterPassword.trim()}
            >
              {unlocking ? "Unlocking…" : "Unlock"}
            </Button>
          </form>
          {unlockError && (
            <div className="bw__toast bw__toast--error">
              <Text size="xs" color="muted">
                {unlockError}
              </Text>
            </div>
          )}
        </div>
      </div>
    );
  }

  // ── Unlocked — show vault ───────────────────────────────────────────────
  return (
    <div className="bw">
      {/* Search bar */}
      <div className="bw__search">
        <Input
          ref={searchRef}
          placeholder="Search vault…"
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          autoFocus
        />
        <div className="bw__search-meta">
          {searching ? (
            <Text size="xs" color="muted">Searching…</Text>
          ) : items.length > 0 ? (
            <Text size="xs" color="muted">
              {filtered.length} item{filtered.length !== 1 ? "s" : ""}
            </Text>
          ) : searchQuery.length >= 2 ? (
            <Text size="xs" color="muted">No results</Text>
          ) : (
            <Text size="xs" color="muted">Type to search your vault</Text>
          )}
        </div>
      </div>

      {/* Error toast */}
      {error && (
        <div className="bw__toast bw__toast--error">
          <Text size="xs" color="muted">{error}</Text>
        </div>
      )}

      {/* Results */}
      <div className="bw__list">
        {grouped.map(([folder, folderItems]) => (
          <div key={folder} className="bw__group">
            <div className="bw__group-header">
              <Text size="xs" color="muted">{folder}</Text>
              <Badge>{folderItems.length}</Badge>
            </div>
            {folderItems.map((item) => (
              <div key={item.id} className="bw__row">
                <div className="bw__item-icon">
                  {item.item_type === "login" ? "🔑" : item.item_type === "secureNote" ? "📝" : item.item_type === "card" ? "💳" : "👤"}
                </div>
                <div className="bw__item-info">
                  <Text size="sm" weight="medium">{item.name}</Text>
                  <Text size="xs" color="muted">
                    {item.username || item.uri || "No details"}
                  </Text>
                </div>
                <div className="bw__item-actions">
                  {item.item_type === "login" && (
                    <>
                      <Button
                        size="sm"
                        variant={
                          copied?.id === item.id && copied?.field === "password"
                            ? "primary"
                            : "ghost"
                        }
                        onClick={() => handleCopy(item, "password")}
                      >
                        {copied?.id === item.id && copied?.field === "password"
                          ? "Copied!"
                          : "Password"}
                      </Button>
                      {item.username && (
                        <Button
                          size="sm"
                          variant={
                            copied?.id === item.id && copied?.field === "username"
                              ? "primary"
                              : "ghost"
                          }
                          onClick={() => handleCopy(item, "username")}
                        >
                          {copied?.id === item.id && copied?.field === "username"
                            ? "Copied!"
                            : "User"}
                        </Button>
                      )}
                      {item.has_totp && (
                        <Button
                          size="sm"
                          variant={
                            copied?.id === item.id && copied?.field === "totp"
                              ? "primary"
                              : "ghost"
                          }
                          onClick={() => handleCopy(item, "totp")}
                        >
                          {copied?.id === item.id && copied?.field === "totp"
                            ? "Copied!"
                            : "TOTP"}
                        </Button>
                      )}
                    </>
                  )}
                </div>
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
