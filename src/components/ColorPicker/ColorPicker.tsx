import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "@base/primitives/icon/Icon";
import { pipette, trash2, copy, save, plus } from "../../lib/icons";
import type { ColorPalette, PickedColor } from "../../lib/types";
import {
  colorPickerLoadPalettes,
  colorPickerSavePalettes,
  colorPickerSampleScreen,
} from "../../lib/tauri";
import { listen } from "@tauri-apps/api/event";
import { rgbToHsl } from "../../lib/color";
import { Button, Kbd } from "../../ui";
import "./ColorPicker.css";

interface ColorPickerProps {
  filter: string;
  onBack: () => void;
  onTrailingChange?: (node: React.ReactNode) => void;
  onQueryChange?: (query: string) => void;
}

function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

export function ColorPicker({
  filter,
  onBack,
  onTrailingChange,
  onQueryChange,
}: ColorPickerProps) {
  const [palettes, setPalettes] = useState<ColorPalette[]>([]);
  const [unsavedColors, setUnsavedColors] = useState<PickedColor[]>([]);
  const [selectedPaletteIndex, setSelectedPaletteIndex] = useState(-1); // -1 = unsaved
  const [copiedHex, setCopiedHex] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  // Track whether filter changes are from user typing vs programmatic
  const programmaticFilterRef = useRef(false);

  // Load palettes on mount
  useEffect(() => {
    colorPickerLoadPalettes().then(setPalettes);
  }, []);

  // Listen for individual color picks from native sampler
  useEffect(() => {
    const unlistenPick = listen<PickedColor>(
      "color-picker-pick",
      (event) => {
        setUnsavedColors((prev) => [...prev, event.payload]);
        setSelectedPaletteIndex(-1);
      },
    );
    // Listen for done event (user pressed Escape)
    const unlistenDone = listen("color-picker-done", () => {
      // Main window is restored by the backend
    });
    return () => {
      unlistenPick.then((fn) => fn());
      unlistenDone.then((fn) => fn());
    };
  }, []);

  // Set default palette name in search bar when unsaved colors arrive
  useEffect(() => {
    if (unsavedColors.length > 0 && selectedPaletteIndex === -1 && onQueryChange) {
      if (!filter || filter === "Untitled Palette" || filter.startsWith("Palette ")) {
        onQueryChange("Untitled Palette");
      }
    }
  }, [unsavedColors.length, selectedPaletteIndex]);

  const handleLaunchPicker = useCallback(async () => {
    await colorPickerSampleScreen();
  }, []);

  const handleSavePalette = useCallback(async () => {
    if (unsavedColors.length === 0) return;
    const name = filter.trim() || `Palette ${palettes.length + 1}`;
    const newPalette: ColorPalette = {
      id: generateId(),
      name,
      colors: [...unsavedColors],
      created_at: Date.now(),
    };
    const updated = [newPalette, ...palettes];
    setPalettes(updated);
    setUnsavedColors([]);
    setSelectedPaletteIndex(0);
    programmaticFilterRef.current = true;
    onQueryChange?.(name);
    await colorPickerSavePalettes(updated);
  }, [unsavedColors, palettes, filter, onQueryChange]);

  const handleDeletePalette = useCallback(
    async (index: number) => {
      const updated = palettes.filter((_, i) => i !== index);
      setPalettes(updated);
      if (updated.length === 0) {
        setSelectedPaletteIndex(-1);
        programmaticFilterRef.current = true;
        onQueryChange?.("");
      } else {
        const newIndex = index >= updated.length ? updated.length - 1 : index;
        setSelectedPaletteIndex(newIndex);
        programmaticFilterRef.current = true;
        onQueryChange?.(updated[newIndex].name);
      }
      await colorPickerSavePalettes(updated);
    },
    [palettes, onQueryChange],
  );

  const handleCopyHex = useCallback(
    async (hex: string) => {
      try {
        await navigator.clipboard.writeText(hex);
        setCopiedHex(hex);
        setTimeout(() => setCopiedHex(null), 1500);
      } catch {
        // Fallback: copy via pbcopy if clipboard API fails
      }
    },
    [],
  );

  const handleRemoveUnsavedColor = useCallback((index: number) => {
    setUnsavedColors((prev) => prev.filter((_, i) => i !== index));
  }, []);

  // When selecting a saved palette, populate search bar with its name
  const handleSelectPalette = useCallback((index: number) => {
    setSelectedPaletteIndex(index);
    if (index >= 0) {
      programmaticFilterRef.current = true;
      onQueryChange?.(palettes[index]?.name ?? "");
    }
  }, [palettes, onQueryChange]);

  // Live-rename palette as user types in the search bar
  useEffect(() => {
    // Skip programmatic filter changes
    if (programmaticFilterRef.current) {
      programmaticFilterRef.current = false;
      return;
    }
    // Only rename when a saved palette is selected
    if (selectedPaletteIndex < 0 || selectedPaletteIndex >= palettes.length) return;
    const currentName = palettes[selectedPaletteIndex].name;
    if (filter === currentName) return;

    const updated = palettes.map((p, i) =>
      i === selectedPaletteIndex ? { ...p, name: filter } : p,
    );
    setPalettes(updated);
    colorPickerSavePalettes(updated);
  }, [filter]);

  // Don't filter palettes when a saved palette is selected (user is renaming)
  const filteredPalettes = useMemo(() => {
    if (!filter || selectedPaletteIndex >= 0) return palettes;
    if (selectedPaletteIndex === -1 && unsavedColors.length > 0) return palettes;
    const q = filter.toLowerCase();
    return palettes.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.colors.some((c) => c.hex.toLowerCase().includes(q)),
    );
  }, [palettes, filter, selectedPaletteIndex, unsavedColors.length]);

  // Selected palette data
  const selectedPalette =
    selectedPaletteIndex === -1 ? null : filteredPalettes[selectedPaletteIndex];
  const displayColors =
    selectedPaletteIndex === -1 ? unsavedColors : selectedPalette?.colors ?? [];

  // Show save button or trash button in search bar trailing area
  useEffect(() => {
    if (unsavedColors.length > 0 && selectedPaletteIndex === -1) {
      // Unsaved: show save button
      onTrailingChange?.(
        <Button size="sm" onClick={handleSavePalette}>
          <Icon icon={save} size="sm" />
          Save
        </Button>,
      );
    } else if (selectedPaletteIndex >= 0 && selectedPalette) {
      // Saved palette selected: show trash button
      onTrailingChange?.(
        <Button
          variant="ghost"
          size="sm"
          onClick={() => handleDeletePalette(selectedPaletteIndex)}
        >
          <Icon icon={trash2} size="sm" />
        </Button>,
      );
    } else {
      onTrailingChange?.(null);
    }
    return () => onTrailingChange?.(null);
  }, [unsavedColors.length, selectedPaletteIndex, selectedPalette, onTrailingChange, handleSavePalette, handleDeletePalette]);

  // Keyboard navigation
  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          {
            const nextIdx = Math.min(selectedPaletteIndex + 1, filteredPalettes.length - 1);
            handleSelectPalette(nextIdx);
          }
          break;
        case "ArrowUp":
          e.preventDefault();
          {
            const prevIdx = Math.max(selectedPaletteIndex - 1, -1);
            if (prevIdx === -1 && unsavedColors.length > 0) {
              setSelectedPaletteIndex(-1);
            } else if (prevIdx >= 0) {
              handleSelectPalette(prevIdx);
            }
          }
          break;
        case "Escape":
          onBack();
          break;
        case "Backspace":
          if (e.metaKey && selectedPaletteIndex >= 0) {
            e.preventDefault();
            handleDeletePalette(selectedPaletteIndex);
          }
          break;
        case "Enter":
          if (unsavedColors.length > 0 && selectedPaletteIndex === -1) {
            if (document.activeElement?.tagName !== "BUTTON") {
              e.preventDefault();
              await handleSavePalette();
            }
          }
          break;
        case "p":
          if (!e.metaKey && !e.ctrlKey) {
            // Don't intercept if user is typing in the name input
            if (document.activeElement?.tagName === "INPUT") return;
            e.preventDefault();
            await handleLaunchPicker();
          }
          break;
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [
    filteredPalettes,
    selectedPaletteIndex,
    unsavedColors,
    onBack,
    handleDeletePalette,
    handleLaunchPicker,
    handleSavePalette,
    handleSelectPalette,
  ]);

  // Scroll selected palette into view
  useEffect(() => {
    const el = listRef.current?.querySelector(".cp-palette-item.selected");
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedPaletteIndex]);

  return (
    <div className="color-picker">
      <div className="cp-body">
        {/* Left: palette list */}
        <div className="cp-list" ref={listRef}>
          <div className="cp-list-header">
            <Button onClick={handleLaunchPicker} style={{ width: "100%" }}>
              <Icon icon={pipette} size="sm" />
              Pick Colors
            </Button>
          </div>

          {/* Unsaved colors entry */}
          {unsavedColors.length > 0 && (
            <div
              className={`cp-palette-item ${selectedPaletteIndex === -1 ? "selected" : ""}`}
              onClick={() => setSelectedPaletteIndex(-1)}
            >
              <div className="cp-palette-name">
                <Icon icon={plus} size="sm" />
                Unsaved Picks
              </div>
              <div className="cp-palette-dots">
                {unsavedColors.slice(0, 8).map((c, i) => (
                  <span
                    key={i}
                    className="cp-dot"
                    style={{ backgroundColor: c.hex }}
                  />
                ))}
                {unsavedColors.length > 8 && (
                  <span className="cp-dot-more">
                    +{unsavedColors.length - 8}
                  </span>
                )}
              </div>
            </div>
          )}

          {/* Saved palettes */}
          {filteredPalettes.length === 0 && unsavedColors.length === 0 ? (
            <div className="cp-empty">
              No palettes yet. Pick some colors to get started!
            </div>
          ) : (
            filteredPalettes.map((palette, i) => (
              <div
                key={palette.id}
                className={`cp-palette-item ${i === selectedPaletteIndex ? "selected" : ""}`}
                onClick={() => handleSelectPalette(i)}
              >
                <div className="cp-palette-name">{palette.name}</div>
                <div className="cp-palette-dots">
                  {palette.colors.slice(0, 8).map((c, j) => (
                    <span
                      key={j}
                      className="cp-dot"
                      style={{ backgroundColor: c.hex }}
                    />
                  ))}
                  {palette.colors.length > 8 && (
                    <span className="cp-dot-more">
                      +{palette.colors.length - 8}
                    </span>
                  )}
                </div>
              </div>
            ))
          )}
        </div>

        {/* Right: palette detail */}
        <div className="cp-detail">
          {displayColors.length === 0 ? (
            <div className="cp-detail-empty">
              {selectedPaletteIndex === -1
                ? "Pick colors from your screen to get started"
                : "Select a palette to view its colors"}
            </div>
          ) : (
            <>
              <div className="cp-colors-grid">
                {displayColors.map((color, i) => {
                  const hsl = rgbToHsl(color.rgb.r, color.rgb.g, color.rgb.b);
                  return (
                    <div key={i} className="cp-color-card">
                      <div
                        className="cp-color-swatch"
                        style={{ backgroundColor: color.hex }}
                        onClick={() => handleCopyHex(color.hex)}
                      >
                        {copiedHex === color.hex && (
                          <span className="cp-copied">Copied!</span>
                        )}
                      </div>
                      <div className="cp-color-values">
                        <button
                          className="cp-color-hex"
                          onClick={() => handleCopyHex(color.hex)}
                        >
                          <Icon icon={copy} size="sm" />
                          {color.hex.toUpperCase()}
                        </button>
                        <span className="cp-color-secondary">
                          rgb({color.rgb.r}, {color.rgb.g}, {color.rgb.b})
                        </span>
                        <span className="cp-color-secondary">
                          hsl({hsl.h}, {hsl.s}%, {hsl.l}%)
                        </span>
                      </div>
                      {selectedPaletteIndex === -1 && (
                        <button
                          className="cp-color-remove"
                          onClick={() => handleRemoveUnsavedColor(i)}
                        >
                          <Icon icon={trash2} size="sm" />
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </div>
      </div>

      <div className="cp-footer">
        <div className="cp-footer-left">
          <span className="cp-footer-count">
            {filteredPalettes.length} palette
            {filteredPalettes.length !== 1 ? "s" : ""}
          </span>
        </div>
        <div className="cp-footer-actions">
          <Kbd>P</Kbd> <span>Pick</span>
          <Kbd>{"\u2191\u2193"}</Kbd> <span>Navigate</span>
          <Kbd>esc</Kbd> <span>Back</span>
        </div>
      </div>
    </div>
  );
}
