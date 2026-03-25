import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "@base/primitives/icon/Icon";
import { copy, trash2, link, typeIcon, externalLink, image } from "../../lib/icons";
import type { ClipboardItem } from "../../lib/types";
import {
  getClipboardHistory,
  clipboardCopy,
  clipboardDelete,
  clipboardClear,
  clipboardGetImage,
  hideWindow,
} from "../../lib/tauri";
import { ActionBar, Button, Kbd, HighlightedText, Select } from "../../ui";
import type { Action } from "../../ui";
import { detectColor } from "../../lib/color";
import { formatTimestamp } from "../../lib/format";
import { substringMatchIndices } from "../../lib/search";
import "./ClipboardManager.css";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const TYPE_OPTIONS = [
  { value: "all", label: "All Types" },
  { value: "text", label: "Text" },
  { value: "url", label: "URLs" },
  { value: "image", label: "Images" },
  { value: "color", label: "Colors" },
];

interface ClipboardManagerProps {
  filter: string;
  onBack: () => void;
  onTrailingChange?: (node: React.ReactNode) => void;
}

export function ClipboardManager({ filter, onBack, onTrailingChange }: ClipboardManagerProps) {
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showActions, setShowActions] = useState(false);
  const [typeFilter, setTypeFilter] = useState("all");
  const [imagePreview, setImagePreview] = useState<string | null>(null);
  const [loadingImage, setLoadingImage] = useState(false);
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});
  const listRef = useRef<HTMLDivElement>(null);

  // Push type filter into the search bar trailing slot
  useEffect(() => {
    onTrailingChange?.(
      <Select
        value={typeFilter}
        options={TYPE_OPTIONS}
        onChange={setTypeFilter}
        variant="pill"
      />,
    );
    return () => onTrailingChange?.(null);
  }, [typeFilter, onTrailingChange]);

  const loadHistory = useCallback(async () => {
    const history = await getClipboardHistory();
    setItems(history);
  }, []);

  useEffect(() => {
    loadHistory();
    const interval = setInterval(loadHistory, 1000);
    return () => clearInterval(interval);
  }, [loadHistory]);

  // Load thumbnails for image items
  useEffect(() => {
    const imageItems = items.filter(
      (item) => item.content_type === "image" && !thumbnails[item.id],
    );
    if (imageItems.length === 0) return;

    let cancelled = false;
    for (const item of imageItems) {
      clipboardGetImage(item.id)
        .then((dataUri) => {
          if (!cancelled) {
            setThumbnails((prev) => ({ ...prev, [item.id]: dataUri }));
          }
        })
        .catch(() => {});
    }
    return () => { cancelled = true; };
  }, [items, thumbnails]);

  const filtered = useMemo(() => {
    let result = items;

    // Type filter
    if (typeFilter !== "all") {
      result = result.filter((item) => {
        if (typeFilter === "color") return detectColor(item.content) !== null;
        return item.content_type === typeFilter;
      });
    }

    // Text search filter
    if (filter) {
      const q = filter.toLowerCase();
      result = result.filter(
        (item) =>
          item.content.toLowerCase().includes(q) ||
          item.preview.toLowerCase().includes(q),
      );
    }

    return result;
  }, [items, filter, typeFilter]);

  const selected = filtered[selectedIndex] ?? null;

  useEffect(() => {
    setSelectedIndex(0);
  }, [filter, typeFilter]);

  // Close actions when selection changes
  useEffect(() => {
    setShowActions(false);
  }, [selectedIndex]);

  // Load image preview when an image item is selected
  useEffect(() => {
    if (selected?.content_type === "image") {
      setLoadingImage(true);
      setImagePreview(null);
      clipboardGetImage(selected.id)
        .then(setImagePreview)
        .catch(() => setImagePreview(null))
        .finally(() => setLoadingImage(false));
    } else {
      setImagePreview(null);
      setLoadingImage(false);
    }
  }, [selected?.id, selected?.content_type]);

  const handleCopy = useCallback(async (item: ClipboardItem) => {
    await clipboardCopy(item.id);
    await hideWindow();
  }, []);

  const handleDelete = useCallback(
    async (item: ClipboardItem) => {
      await clipboardDelete(item.id);
      await loadHistory();
    },
    [loadHistory],
  );

  const handleClear = async () => {
    await clipboardClear();
    await loadHistory();
  };

  const handlePasteToFrontApp = useCallback(async (item: ClipboardItem) => {
    await clipboardCopy(item.id);
    await hideWindow();
  }, []);

  const handleOpenUrl = useCallback((item: ClipboardItem) => {
    window.open(item.content, "_blank");
  }, []);

  const actions: Action[] = useMemo(() => {
    if (!selected) return [];
    const list: Action[] = [
      {
        id: "copy",
        label: selected.content_type === "image" ? "Copy Image" : "Copy to Clipboard",
        icon: <Icon icon={copy} size="sm" />,
        shortcut: ["↵"],
        action: () => handleCopy(selected),
      },
      {
        id: "paste",
        label: "Paste to App",
        icon: <Icon icon={externalLink} size="sm" />,
        shortcut: ["⇧", "↵"],
        action: () => handlePasteToFrontApp(selected),
      },
    ];

    if (selected.content_type === "url") {
      list.push({
        id: "open",
        label: "Open URL",
        icon: <Icon icon={link} size="sm" />,
        shortcut: ["⌘", "O"],
        action: () => handleOpenUrl(selected),
      });
    }

    list.push({
      id: "delete",
      label: "Delete Entry",
      icon: <Icon icon={trash2} size="sm" />,
      shortcut: ["⌘", "⌫"],
      action: () => handleDelete(selected),
    });

    return list;
  }, [selected, handleCopy, handlePasteToFrontApp, handleOpenUrl, handleDelete]);

  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      // Toggle actions panel with ⌘K
      if (e.metaKey && e.key === "k") {
        e.preventDefault();
        setShowActions((prev) => !prev);
        return;
      }

      // Let ActionBar handle its own keys when open
      if (showActions) return;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          break;
        case "Enter":
          if (e.shiftKey && selected) {
            await handlePasteToFrontApp(selected);
          } else if (selected) {
            await handleCopy(selected);
          }
          break;
        case "Escape":
          onBack();
          break;
        case "Backspace":
          if (e.metaKey && selected) {
            e.preventDefault();
            await handleDelete(selected);
          }
          break;
        case "o":
          if (e.metaKey && selected?.content_type === "url") {
            e.preventDefault();
            handleOpenUrl(selected);
          }
          break;
      }
    };

    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [filtered, selected, selectedIndex, onBack, showActions, handleCopy, handleDelete, handlePasteToFrontApp, handleOpenUrl]);

  // Scroll selected item into view
  useEffect(() => {
    const el = listRef.current?.querySelector(".clip-item.selected");
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const renderItemIcon = (item: ClipboardItem) => {
    if (item.content_type === "image") {
      const thumb = thumbnails[item.id];
      if (thumb) {
        return <img src={thumb} alt="" className="clip-item-thumb" />;
      }
      return <Icon icon={image} size="sm" />;
    }
    if (item.content_type === "url") {
      return <Icon icon={link} size="sm" />;
    }
    if (detectColor(item.content)) {
      return (
        <span
          className="clip-color-swatch"
          style={{ backgroundColor: detectColor(item.content)! }}
        />
      );
    }
    return <Icon icon={typeIcon} size="sm" />;
  };

  const renderPreviewType = () => {
    if (!selected) return null;
    if (selected.content_type === "image") {
      return (
        <>
          <Icon icon={image} size="sm" />
          Image
        </>
      );
    }
    if (selected.content_type === "url") {
      return (
        <>
          <Icon icon={link} size="sm" />
          URL
        </>
      );
    }
    if (detectColor(selected.content)) {
      return (
        <>
          <span
            className="clip-color-swatch clip-color-swatch--sm"
            style={{ backgroundColor: detectColor(selected.content)! }}
          />
          Color
        </>
      );
    }
    return (
      <>
        <Icon icon={typeIcon} size="sm" />
        Text
      </>
    );
  };

  return (
    <div className="clipboard-manager">
      <div className="clip-body">
        <div className="clip-list" ref={listRef}>
          {filtered.length === 0 ? (
            <div className="clip-empty">
              {items.length === 0
                ? "No clipboard history yet"
                : "No matching items"}
            </div>
          ) : (
            filtered.map((item, i) => (
              <div
                key={item.id}
                className={`clip-item ${i === selectedIndex ? "selected" : ""}`}
                onClick={() => setSelectedIndex(i)}
                onDoubleClick={() => handleCopy(item)}
              >
                <div className="clip-item-icon">
                  {renderItemIcon(item)}
                </div>
                <div className="clip-item-info">
                  <div className="clip-item-preview">
                    <HighlightedText
                      text={item.preview}
                      indices={substringMatchIndices(item.preview, filter)}
                    />
                  </div>
                  <div className="clip-item-time">
                    {formatTimestamp(item.timestamp)}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>

        <div className="clip-preview">
          {selected ? (
            <>
              <div className="clip-preview-header">
                <span className="clip-preview-type">
                  {renderPreviewType()}
                </span>
                <div className="clip-preview-actions">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => handleCopy(selected)}
                  >
                    <Icon icon={copy} size="sm" /> Copy
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleDelete(selected)}
                  >
                    <Icon icon={trash2} size="sm" />
                  </Button>
                </div>
              </div>
              <div className="clip-preview-body">
                {selected.content_type === "image" ? (
                  <div className="clip-image-preview">
                    {loadingImage ? (
                      <div className="clip-image-loading">Loading image…</div>
                    ) : imagePreview ? (
                      <>
                        <img
                          src={imagePreview}
                          alt="Clipboard image"
                          className="clip-image-img"
                        />
                        {selected.metadata && (
                          <table className="clip-image-meta-table">
                            <tbody>
                              <tr>
                                <td>Dimensions</td>
                                <td>{selected.metadata.width} × {selected.metadata.height}</td>
                              </tr>
                              <tr>
                                <td>Size</td>
                                <td>{formatSize(selected.metadata.size_bytes)}</td>
                              </tr>
                              {selected.metadata.source_app && (
                                <tr>
                                  <td>Source</td>
                                  <td>{selected.metadata.source_app}</td>
                                </tr>
                              )}
                            </tbody>
                          </table>
                        )}
                      </>
                    ) : (
                      <div className="clip-image-loading">Image not available</div>
                    )}
                  </div>
                ) : (
                  <div className="clip-preview-content">
                    <pre>{selected.content}</pre>
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="clip-preview-empty">Select an item to preview</div>
          )}
        </div>
      </div>

      <div className="clip-footer">
        <div className="clip-footer-left">
          <span className="clip-footer-count">
            {filtered.length} item{filtered.length !== 1 ? "s" : ""}
          </span>
          <Button variant="ghost" size="sm" onClick={handleClear}>
            <Icon icon={trash2} size="sm" /> Clear All
          </Button>
        </div>
        <div className="clip-footer-actions">
          <Kbd>{"\u2191\u2193"}</Kbd> <span>Navigate</span>
          <Kbd>{"\u21B5"}</Kbd> <span>Copy</span>
          <Kbd>esc</Kbd> <span>Back</span>
          <span className="clip-footer-divider" />
          <div className="clip-footer-actions-anchor">
            <Kbd>⌘</Kbd><Kbd>K</Kbd> <span>Actions</span>
            <ActionBar
              actions={actions}
              open={showActions}
              onClose={() => setShowActions(false)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
