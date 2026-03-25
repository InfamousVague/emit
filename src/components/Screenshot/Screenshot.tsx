import { useCallback, useEffect, useMemo, useState } from "react";
import { Icon } from "@base/primitives/icon/Icon";
import { scan, appWindow, monitor, copy, trash2, arrowLeft } from "../../lib/icons";
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
import { Button, Kbd } from "../../ui";
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
          <Button size="sm" variant="ghost" onClick={() => setDetailItem(null)}>
            <Icon icon={arrowLeft} size="sm" /> Back
          </Button>
          <div className="screenshot-detail-header-actions">
            <Button
              size="sm"
              variant="ghost"
              onClick={(e) => handleCopy(e, detailItem.id)}
            >
              <Icon icon={copy} size="sm" /> Copy
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={(e) => handleDelete(e, detailItem.id)}
            >
              <Icon icon={trash2} size="sm" /> Delete
            </Button>
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
              <Icon icon={scan} size="sm" className="screenshot-option-icon" />
              <div className="screenshot-option-text">
                <span className="screenshot-option-label">Capture Region</span>
                <span className="screenshot-option-desc">Select an area to capture</span>
              </div>
            </div>
            <Kbd>{"\u2318\u21E7 4"}</Kbd>
          </button>
          <button className="screenshot-option-row" onClick={() => screenshotCaptureWindow()}>
            <div className="screenshot-option-info">
              <Icon icon={appWindow} size="sm" className="screenshot-option-icon" />
              <div className="screenshot-option-text">
                <span className="screenshot-option-label">Capture Window</span>
                <span className="screenshot-option-desc">Click a window to capture with shadow</span>
              </div>
            </div>
            <Kbd>{"\u2318\u21E7 5"}</Kbd>
          </button>
          <button className="screenshot-option-row" onClick={() => screenshotCaptureScreen()}>
            <div className="screenshot-option-info">
              <Icon icon={monitor} size="sm" className="screenshot-option-icon" />
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
                    <Button
                      size="sm"
                      variant="ghost"
                      iconOnly
                      icon={copy}
                      aria-label="Copy"
                      onClick={(e) => handleCopy(e, item.id)}
                    />
                    <Button
                      size="sm"
                      variant="ghost"
                      iconOnly
                      icon={trash2}
                      aria-label="Delete"
                      onClick={(e) => handleDelete(e, item.id)}
                    />
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
