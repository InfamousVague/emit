import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Selection,
  AppWindow,
  Desktop,
  Copy,
  Trash,
  ArrowLeft,
} from "@phosphor-icons/react";
import type { ScreenshotItem } from "../../lib/types";
import {
  screenshotCaptureRegion,
  screenshotCaptureWindow,
  screenshotCaptureScreen,
  screenshotList,
  screenshotDelete,
  screenshotCopy,
  screenshotGetImage,
} from "../../lib/tauri";
import { listen } from "@tauri-apps/api/event";
import { Kbd } from "../../ui";
import "./Screenshot.css";

interface ScreenshotProps {
  filter: string;
  onBack: () => void;
  onTrailingChange?: (node: React.ReactNode) => void;
}

export function Screenshot({ filter, onBack, onTrailingChange }: ScreenshotProps) {
  const [items, setItems] = useState<ScreenshotItem[]>([]);
  const [detailItem, setDetailItem] = useState<ScreenshotItem | null>(null);
  const [detailSrc, setDetailSrc] = useState<string>("");
  const [thumbCache, setThumbCache] = useState<Record<string, string>>({});

  const loadItems = useCallback(async () => {
    const list = await screenshotList();
    setItems(list);
  }, []);

  useEffect(() => {
    loadItems();
  }, [loadItems]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ScreenshotItem>("screenshot-captured", () => {
      loadItems();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [loadItems]);

  useEffect(() => {
    for (const item of items) {
      if (!thumbCache[item.id]) {
        screenshotGetImage(item.thumbnail_path).then((src) => {
          setThumbCache((prev) => ({ ...prev, [item.id]: src }));
        });
      }
    }
  }, [items, thumbCache]);

  // Clear trailing on unmount
  useEffect(() => {
    return () => onTrailingChange?.(null);
  }, [onTrailingChange]);

  const filtered = useMemo(() => {
    if (!filter) return items;
    const q = filter.toLowerCase();
    return items.filter(
      (item) =>
        (item.source_app ?? "").toLowerCase().includes(q) ||
        new Date(item.timestamp).toLocaleDateString().includes(q),
    );
  }, [items, filter]);

  const handleCopy = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    await screenshotCopy(id);
  };

  const handleDelete = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    await screenshotDelete(id);
    setItems((prev) => prev.filter((i) => i.id !== id));
    if (detailItem?.id === id) {
      setDetailItem(null);
    }
  };

  const openDetail = async (item: ScreenshotItem) => {
    setDetailItem(item);
    const src = await screenshotGetImage(item.path);
    setDetailSrc(src);
  };

  const formatTime = (ts: number) => {
    const d = new Date(ts);
    return d.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  if (detailItem) {
    return (
      <div className="screenshot">
        <div className="screenshot-detail-header">
          <button
            className="screenshot-action-btn"
            onClick={() => setDetailItem(null)}
          >
            <ArrowLeft size={12} weight="regular" /> Back
          </button>
          <div className="screenshot-detail-header-actions">
            <button
              className="screenshot-action-btn"
              onClick={(e) => handleCopy(e, detailItem.id)}
            >
              <Copy size={12} weight="regular" /> Copy
            </button>
            <button
              className="screenshot-action-btn"
              onClick={(e) => handleDelete(e, detailItem.id)}
            >
              <Trash size={12} weight="regular" /> Delete
            </button>
          </div>
        </div>
        <div className="screenshot-detail">
          {detailSrc && (
            <img
              className="screenshot-detail-img"
              src={detailSrc}
              alt="Screenshot"
            />
          )}
          <div className="screenshot-detail-info">
            {detailItem.width}&times;{detailItem.height}
            {detailItem.source_app && ` \u2014 ${detailItem.source_app}`}
            {" \u2014 "}
            {formatTime(detailItem.timestamp)}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="screenshot">
      <div className="screenshot-body">
        {/* Capture options */}
        <div className="screenshot-options">
          <button className="screenshot-option-row" onClick={() => screenshotCaptureRegion()}>
            <div className="screenshot-option-info">
              <Selection size={16} weight="regular" className="screenshot-option-icon" />
              <div className="screenshot-option-text">
                <span className="screenshot-option-label">Capture Region</span>
                <span className="screenshot-option-desc">Select an area to capture</span>
              </div>
            </div>
            <Kbd>{"\u2318\u21E7 4"}</Kbd>
          </button>
          <button className="screenshot-option-row" onClick={() => screenshotCaptureWindow()}>
            <div className="screenshot-option-info">
              <AppWindow size={16} weight="regular" className="screenshot-option-icon" />
              <div className="screenshot-option-text">
                <span className="screenshot-option-label">Capture Window</span>
                <span className="screenshot-option-desc">Click a window to capture with shadow</span>
              </div>
            </div>
            <Kbd>{"\u2318\u21E7 5"}</Kbd>
          </button>
          <button className="screenshot-option-row" onClick={() => screenshotCaptureScreen()}>
            <div className="screenshot-option-info">
              <Desktop size={16} weight="regular" className="screenshot-option-icon" />
              <div className="screenshot-option-text">
                <span className="screenshot-option-label">Capture Screen</span>
                <span className="screenshot-option-desc">Capture the full screen</span>
              </div>
            </div>
            <Kbd>{"\u2318\u21E7 3"}</Kbd>
          </button>
        </div>

        {/* Gallery */}
        {filtered.length > 0 && (
          <>
            <div className="screenshot-section-label">Gallery</div>
            <div className="screenshot-gallery">
              {filtered.map((item) => (
                <div
                  key={item.id}
                  className="screenshot-thumb"
                  onClick={() => openDetail(item)}
                >
                  {thumbCache[item.id] && (
                    <img src={thumbCache[item.id]} alt="" />
                  )}
                  <div className="screenshot-thumb-overlay">
                    <button
                      className="screenshot-thumb-btn"
                      onClick={(e) => handleCopy(e, item.id)}
                    >
                      <Copy size={14} weight="regular" />
                    </button>
                    <button
                      className="screenshot-thumb-btn"
                      onClick={(e) => handleDelete(e, item.id)}
                    >
                      <Trash size={14} weight="regular" />
                    </button>
                  </div>
                  <div className="screenshot-thumb-meta">
                    {item.source_app ?? formatTime(item.timestamp)}
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
