import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Copy,
  Trash,
  LockSimple,
  LockSimpleOpen,
  Eye,
  EyeSlash,
  Check,
  TrashSimple,
} from "@phosphor-icons/react";
import type { PasswordHistoryEntry } from "../../lib/types";
import {
  pwgenHasVault,
  pwgenSetup,
  pwgenUnlock,
  pwgenLock,
  pwgenIsUnlocked,
  pwgenGetHistory,
  pwgenDeleteHistoryEntry,
  pwgenClearHistory,
  pwgenCopyPassword,
} from "../../lib/tauri";
import { Button, Kbd } from "../../ui";
import { PasswordGeneratorWidget } from "./PasswordGeneratorWidget";
import "./PasswordGenerator.css";

interface PasswordGeneratorProps {
  filter: string;
  onBack: () => void;
  onTrailingChange?: (node: React.ReactNode) => void;
}

function formatTimestamp(ts: number): string {
  const d = new Date(ts);
  const now = new Date();
  const diff = now.getTime() - d.getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "Just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  if (days < 7) return `${days}d ago`;
  return d.toLocaleDateString();
}

export function PasswordGenerator({ filter, onBack, onTrailingChange }: PasswordGeneratorProps) {
  const [hasVault, setHasVault] = useState<boolean | null>(null);
  const [unlocked, setUnlocked] = useState(false);
  const [history, setHistory] = useState<PasswordHistoryEntry[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [error, setError] = useState("");
  const [shake, setShake] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [revealedId, setRevealedId] = useState<string | null>(null);

  // Setup form
  const [setupPassword, setSetupPassword] = useState("");
  const [setupConfirm, setSetupConfirm] = useState("");

  const listRef = useRef<HTMLDivElement>(null);

  // Check vault state
  const checkState = useCallback(async () => {
    const exists = await pwgenHasVault();
    setHasVault(exists);
    if (exists) {
      const isUnlocked = await pwgenIsUnlocked();
      setUnlocked(isUnlocked);
      if (isUnlocked) {
        const list = await pwgenGetHistory();
        setHistory(list);
      }
    }
  }, []);

  useEffect(() => {
    checkState();
    const interval = setInterval(async () => {
      if (hasVault) {
        const isUnlocked = await pwgenIsUnlocked();
        if (!isUnlocked && unlocked) {
          setUnlocked(false);
          setHistory([]);
        }
        setUnlocked(isUnlocked);
      }
    }, 5000);
    return () => clearInterval(interval);
  }, [checkState, hasVault, unlocked]);

  // Cleanup trailing on unmount
  useEffect(() => {
    return () => onTrailingChange?.(null);
  }, [onTrailingChange]);

  // Reload history when a password is generated
  const handleGenerated = useCallback(async () => {
    if (unlocked) {
      const list = await pwgenGetHistory();
      setHistory(list);
    }
  }, [unlocked]);

  const filtered = useMemo(() => {
    if (!filter) return history;
    const q = filter.toLowerCase();
    return history.filter(
      (e) =>
        (e.label?.toLowerCase().includes(q) ?? false) ||
        e.mode.toLowerCase().includes(q),
    );
  }, [history, filter]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [filter]);

  // Setup vault
  const handleSetup = async () => {
    setError("");
    if (setupPassword.length < 8) {
      setError("Password must be at least 8 characters");
      return;
    }
    if (setupPassword !== setupConfirm) {
      setError("Passwords do not match");
      return;
    }
    try {
      await pwgenSetup(setupPassword);
      setSetupPassword("");
      setSetupConfirm("");
      await checkState();
    } catch (e) {
      setError(String(e));
    }
  };

  // Unlock
  const handleUnlock = async (password: string) => {
    setError("");
    try {
      await pwgenUnlock(password);
      setUnlocked(true);
      const list = await pwgenGetHistory();
      setHistory(list);
    } catch {
      setError("Wrong password");
      setShake(true);
      setTimeout(() => setShake(false), 300);
    }
  };

  // Lock
  const handleLock = async () => {
    await pwgenLock();
    setUnlocked(false);
    setHistory([]);
  };

  // Copy password from history
  const handleCopy = async (id: string) => {
    try {
      const pw = await pwgenCopyPassword(id);
      await navigator.clipboard.writeText(pw);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 1500);
    } catch {
      // ignore
    }
  };

  // Delete history entry
  const handleDelete = async (id: string) => {
    await pwgenDeleteHistoryEntry(id);
    const list = await pwgenGetHistory();
    setHistory(list);
  };

  // Clear all history
  const handleClearAll = async () => {
    await pwgenClearHistory();
    setHistory([]);
  };

  // Keyboard navigation
  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          break;
        case "Enter": {
          const entry = filtered[selectedIndex];
          if (entry && unlocked) {
            await handleCopy(entry.id);
          }
          break;
        }
        case "Escape":
          onBack();
          break;
        case "Backspace":
          if (e.metaKey) {
            const entry = filtered[selectedIndex];
            if (entry && unlocked) {
              e.preventDefault();
              await handleDelete(entry.id);
            }
          }
          break;
      }
    };

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [filtered, selectedIndex, onBack, unlocked]);

  // Scroll selected into view
  useEffect(() => {
    const el = listRef.current?.querySelector(".pm-item.selected");
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  // ── Setup screen ──
  if (hasVault === false) {
    return (
      <div className="password-manager">
        <div className="pm-setup">
          <LockSimple size={32} weight="regular" style={{ opacity: 0.5 }} />
          <div className="pm-setup-title">Create Your Vault</div>
          <div className="pm-setup-desc">
            Choose a master password to encrypt your password history. Make it strong — this is the only password you'll need to remember.
          </div>
          <div className={`pm-setup-form ${shake ? "pm-shake" : ""}`}>
            <input
              type="password"
              placeholder="Master password"
              value={setupPassword}
              onChange={(e) => setSetupPassword(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && document.getElementById("pg-confirm-input")?.focus()}
              autoFocus
            />
            <input
              id="pg-confirm-input"
              type="password"
              placeholder="Confirm password"
              value={setupConfirm}
              onChange={(e) => setSetupConfirm(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSetup()}
            />
            {error && <div className="pm-error">{error}</div>}
            <Button variant="primary" size="sm" onClick={handleSetup}>
              Create Vault
            </Button>
          </div>
        </div>
      </div>
    );
  }

  // ── Loading ──
  if (hasVault === null) {
    return (
      <div className="password-manager">
        <div className="pm-locked">
          <div className="pm-locked-text">Loading...</div>
        </div>
      </div>
    );
  }

  // ── Main view ──
  return (
    <div className="password-manager">
      <div className="pm-body">
        {/* Left: Generator */}
        <div className="pm-list" style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
          <PasswordGeneratorWidget onGenerated={handleGenerated} />
        </div>

        {/* Right: History */}
        <div className="pm-detail" ref={listRef}>
          {!unlocked ? (
            <div className="pm-locked">
              <LockSimple size={24} weight="regular" className="pm-locked-icon" />
              <div className="pm-locked-text">Unlock to view history</div>
              <UnlockForm onUnlock={handleUnlock} error={error} shake={shake} />
            </div>
          ) : filtered.length === 0 ? (
            <div className="pm-detail-empty">
              {history.length === 0 ? "No passwords generated yet" : "No matching entries"}
            </div>
          ) : (
            <div className="pg-history-list">
              {filtered.map((entry, i) => (
                <div
                  key={entry.id}
                  className={`pm-item ${i === selectedIndex ? "selected" : ""}`}
                  onClick={() => setSelectedIndex(i)}
                >
                  <div className="pm-item-info">
                    <div className="pm-item-name">
                      {revealedId === entry.id ? entry.password : "••••••••••••"}
                    </div>
                    <div className="pm-item-url">
                      {entry.label ?? entry.mode} · {formatTimestamp(entry.generated_at)}
                    </div>
                  </div>
                  <div className="pm-item-badges">
                    <button
                      className="pm-copy-btn"
                      onClick={(e) => { e.stopPropagation(); setRevealedId(revealedId === entry.id ? null : entry.id); }}
                      title={revealedId === entry.id ? "Hide" : "Reveal"}
                    >
                      {revealedId === entry.id ? <EyeSlash size={14} /> : <Eye size={14} />}
                    </button>
                    <button
                      className={`pm-copy-btn ${copiedId === entry.id ? "copied" : ""}`}
                      onClick={(e) => { e.stopPropagation(); handleCopy(entry.id); }}
                      title="Copy"
                    >
                      {copiedId === entry.id ? <Check size={14} /> : <Copy size={14} />}
                    </button>
                    <button
                      className="pm-copy-btn"
                      onClick={(e) => { e.stopPropagation(); handleDelete(entry.id); }}
                      title="Delete"
                    >
                      <Trash size={14} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="pm-footer">
        <div className="pm-footer-left">
          <span className="pm-footer-count">
            {filtered.length} password{filtered.length !== 1 ? "s" : ""}
          </span>
          <span className={`pm-footer-lock-status ${unlocked ? "unlocked" : "locked"}`}>
            {unlocked ? (
              <><LockSimpleOpen size={10} /> Unlocked</>
            ) : (
              <><LockSimple size={10} /> Locked</>
            )}
          </span>
          {unlocked && (
            <>
              <Button variant="ghost" size="sm" onClick={handleLock}>
                <LockSimple size={12} /> Lock
              </Button>
              {history.length > 0 && (
                <Button variant="ghost" size="sm" onClick={handleClearAll}>
                  <TrashSimple size={12} /> Clear
                </Button>
              )}
            </>
          )}
        </div>
        <div className="pm-footer-actions">
          <Kbd>{"\u2191\u2193"}</Kbd> <span>Navigate</span>
          <Kbd>{"\u21B5"}</Kbd> <span>Copy</span>
          <Kbd>esc</Kbd> <span>Back</span>
        </div>
      </div>
    </div>
  );
}

// ── Inline unlock form ──

function UnlockForm({ onUnlock, error, shake }: { onUnlock: (pw: string) => void; error: string; shake: boolean }) {
  const [password, setPassword] = useState("");

  return (
    <div className={`pm-setup-form ${shake ? "pm-shake" : ""}`} style={{ marginTop: "var(--space-md)" }}>
      <input
        type="password"
        placeholder="Master password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.stopPropagation();
            onUnlock(password);
          }
        }}
        autoFocus
      />
      {error && <div className="pm-error">{error}</div>}
      <Button variant="primary" size="sm" onClick={() => onUnlock(password)}>
        Unlock
      </Button>
    </div>
  );
}
