import { useCallback, useEffect, useRef, useState } from "react";
import { Icon } from "@base/primitives/icon/Icon";
import { refreshCw, copy, check } from "../../lib/icons";
import type { GeneratePasswordOpts } from "../../lib/types";
import { pwgenGenerate, pwgenSaveToHistory } from "../../lib/tauri";

interface PasswordGeneratorWidgetProps {
  onGenerated?: () => void;
}

function passwordStrength(pw: string): { level: number; label: string } {
  let score = 0;
  if (pw.length >= 8) score++;
  if (pw.length >= 12) score++;
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) score++;
  if (/\d/.test(pw)) score++;
  if (/[^a-zA-Z0-9]/.test(pw)) score++;

  if (score <= 1) return { level: 1, label: "Weak" };
  if (score <= 2) return { level: 2, label: "Fair" };
  if (score <= 3) return { level: 3, label: "Good" };
  return { level: 4, label: "Strong" };
}

const STRENGTH_CLASSES = ["weak", "fair", "good", "strong"];

export function PasswordGeneratorWidget({ onGenerated }: PasswordGeneratorWidgetProps) {
  const [password, setPassword] = useState("");
  const [length, setLength] = useState(20);
  const [uppercase, setUppercase] = useState(true);
  const [lowercase, setLowercase] = useState(true);
  const [numbers, setNumbers] = useState(true);
  const [symbols, setSymbols] = useState(true);
  const [passphrase, setPassphrase] = useState(false);
  const [wordCount, setWordCount] = useState(4);
  const [separator, setSeparator] = useState("-");
  const [copied, setCopied] = useState(false);

  // Track current opts for save-on-copy
  const optsRef = useRef({ passphrase, length, wordCount });
  optsRef.current = { passphrase, length, wordCount };

  const generate = useCallback(async () => {
    const opts: GeneratePasswordOpts = {
      length,
      uppercase,
      lowercase,
      numbers,
      symbols,
      passphrase,
      word_count: passphrase ? wordCount : undefined,
      separator: passphrase ? separator : undefined,
    };
    try {
      const pw = await pwgenGenerate(opts);
      setPassword(pw);
    } catch {
      // fallback
    }
  }, [length, uppercase, lowercase, numbers, symbols, passphrase, wordCount, separator]);

  useEffect(() => {
    generate();
  }, [generate]);

  const handleCopy = async () => {
    if (!password) return;
    await navigator.clipboard.writeText(password);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);

    // Save to history on copy
    const { passphrase: isPassphrase, length: len, wordCount: wc } = optsRef.current;
    const mode = isPassphrase ? "passphrase" : "random";
    const entryLength = isPassphrase ? wc : len;
    try {
      await pwgenSaveToHistory(password, mode, entryLength);
      onGenerated?.();
    } catch {
      // vault may be locked — silently skip save
    }
  };

  const strength = passwordStrength(password);

  return (
    <div className="pg-generator">
      <div className="pm-field">
        <div className="pm-field-label">Generated Password</div>
        <div className="pm-field-value">
          <code style={{ fontFamily: "var(--font-family-mono)", fontSize: "13px", wordBreak: "break-all", whiteSpace: "pre-wrap" }}>
            {password}
          </code>
          <button className={`pm-copy-btn ${copied ? "copied" : ""}`} onClick={handleCopy} title="Copy">
            {copied ? <Icon icon={check} size="sm" /> : <Icon icon={copy} size="sm" />}
          </button>
          <button className="pm-copy-btn" onClick={generate} title="Regenerate">
            <Icon icon={refreshCw} size="sm" />
          </button>
        </div>
        <div className="pm-strength">
          {[0, 1, 2, 3].map((i) => (
            <div
              key={i}
              className={`pm-strength-bar ${i < strength.level ? `active ${STRENGTH_CLASSES[strength.level - 1]}` : ""}`}
            />
          ))}
        </div>
        <div style={{ fontSize: "var(--font-size-xs)", color: "var(--color-text-tertiary)", marginTop: "2px" }}>
          {strength.label}
        </div>
      </div>

      <div className="pm-edit-field">
        <label style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
          <input
            type="checkbox"
            checked={passphrase}
            onChange={(e) => setPassphrase(e.target.checked)}
            style={{ accentColor: "var(--color-border-selected)" }}
          />
          Passphrase mode
        </label>
      </div>

      {passphrase ? (
        <>
          <div className="pm-edit-field">
            <label>Words ({wordCount})</label>
            <input
              type="range"
              min={3}
              max={8}
              value={wordCount}
              onChange={(e) => setWordCount(Number(e.target.value))}
            />
          </div>
          <div className="pm-edit-field">
            <label>Separator</label>
            <input
              type="text"
              value={separator}
              onChange={(e) => setSeparator(e.target.value)}
              maxLength={3}
              style={{ width: "60px" }}
            />
          </div>
        </>
      ) : (
        <>
          <div className="pm-edit-field">
            <label>Length ({length})</label>
            <input
              type="range"
              min={8}
              max={64}
              value={length}
              onChange={(e) => setLength(Number(e.target.value))}
            />
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: "var(--space-md)" }}>
            {[
              { label: "A-Z", checked: uppercase, set: setUppercase },
              { label: "a-z", checked: lowercase, set: setLowercase },
              { label: "0-9", checked: numbers, set: setNumbers },
              { label: "!@#", checked: symbols, set: setSymbols },
            ].map(({ label, checked, set }) => (
              <label key={label} style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "var(--font-size-xs)", color: "var(--color-text-secondary)", cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(e) => set(e.target.checked)}
                  style={{ accentColor: "var(--color-border-selected)" }}
                />
                {label}
              </label>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
