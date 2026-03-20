import { useCallback, useEffect, useRef, useState } from "react";
import type { SelectOption } from "../lib/types";
import { resolveParamOptions } from "../lib/tauri";
import type { CommandParserState } from "./useCommandParser";

interface UseCommandModeOptions {
  commandParser: CommandParserState;
  query: string;
  setQuery: (q: string) => void;
  onCommandSelect: (command: import("../lib/types").CommandDefinition, initVals?: Record<string, unknown>) => void;
}

export function useCommandMode({
  commandParser,
  query,
  setQuery,
  onCommandSelect,
}: UseCommandModeOptions) {
  const [autocompleteOptions, setAutocompleteOptions] = useState<SelectOption[]>([]);
  const [autocompleteIndex, setAutocompleteIndex] = useState(0);
  const autocompleteDebounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const isPickerParam = !!(
    commandParser.currentParam &&
    ["DatabasePicker", "PagePicker", "People", "DynamicSelect"].includes(
      commandParser.currentParam.param_type.type,
    )
  );

  const showAutocomplete =
    commandParser.phase === "args" &&
    isPickerParam &&
    autocompleteOptions.length > 0;

  // Fetch autocomplete options when currentParam or query changes
  useEffect(() => {
    if (!commandParser.lockedCommand || !commandParser.currentParam) {
      setAutocompleteOptions([]);
      return;
    }

    const paramType = commandParser.currentParam.param_type.type;
    if (
      !["DatabasePicker", "PagePicker", "People", "DynamicSelect"].includes(paramType)
    ) {
      setAutocompleteOptions([]);
      return;
    }

    if (autocompleteDebounceRef.current)
      clearTimeout(autocompleteDebounceRef.current);

    autocompleteDebounceRef.current = setTimeout(async () => {
      try {
        const options = await resolveParamOptions(
          commandParser.lockedCommand!.id,
          commandParser.currentParam!.id,
          commandParser.currentParamQuery,
        );
        setAutocompleteOptions(options);
        setAutocompleteIndex(0);
      } catch {
        setAutocompleteOptions([]);
      }
    }, 150);

    return () => {
      if (autocompleteDebounceRef.current)
        clearTimeout(autocompleteDebounceRef.current);
    };
  }, [
    commandParser.lockedCommand,
    commandParser.currentParam,
    commandParser.currentParamQuery,
  ]);

  const handleCommandKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      const { commandSuggestions, phase, labelRanges } = commandParser;

      if (phase === "selecting") {
        if (e.key === "Tab" || e.key === "Enter") {
          e.preventDefault();
          const result = commandParser.accept();
          if (result) {
            onCommandSelect(result.command, result.initialValues);
          }
        } else if (e.key === "Escape") {
          e.preventDefault();
          setQuery("");
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          commandParser.setSelectedIndex(
            Math.min(commandParser.selectedIndex + 1, commandSuggestions.length - 1),
          );
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          commandParser.setSelectedIndex(
            Math.max(commandParser.selectedIndex - 1, 0),
          );
        }
      } else {
        // Phase "args"
        if (showAutocomplete) {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setAutocompleteIndex((i) => Math.min(i + 1, autocompleteOptions.length - 1));
            return;
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setAutocompleteIndex((i) => Math.max(i - 1, 0));
            return;
          } else if (e.key === "Tab" || e.key === "Enter") {
            e.preventDefault();
            const opt = autocompleteOptions[autocompleteIndex];
            if (opt) {
              commandParser.selectAutocomplete(opt.label, opt.value);
              setAutocompleteOptions([]);
            }
            return;
          }
        }

        // Backspace: delete entire label as a unit
        if (e.key === "Backspace" && labelRanges.length > 0) {
          const cursorPos = query.length;
          for (const range of labelRanges) {
            if (cursorPos > range.start && cursorPos <= range.end) {
              e.preventDefault();
              setQuery(query.slice(0, range.start));
              return;
            }
          }
        }

        if (e.key === "Enter") {
          e.preventDefault();
          const result = commandParser.accept();
          if (result) {
            onCommandSelect(result.command, result.initialValues);
          }
        } else if (e.key === "Tab") {
          e.preventDefault();
          commandParser.advanceParam();
        } else if (e.key === "Escape") {
          e.preventDefault();
          setQuery("");
        }
      }
    },
    [commandParser, onCommandSelect, setQuery, query, showAutocomplete, autocompleteOptions, autocompleteIndex],
  );

  return {
    autocompleteOptions,
    autocompleteIndex,
    showAutocomplete,
    handleCommandKeyDown,
    setAutocompleteOptions,
  };
}
