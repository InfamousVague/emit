import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import type { CommandDefinition, ParamDefinition } from "../lib/types";
import { searchCommands } from "../lib/tauri";

export interface LabelRange {
  start: number;
  end: number;
}

export interface CommandParserState {
  phase: "selecting" | "args";
  commandSuggestions: CommandDefinition[];
  selectedIndex: number;
  ghostText: string;
  accept: () => {
    command: CommandDefinition;
    initialValues: Record<string, unknown>;
  } | null;
  setSelectedIndex: (i: number) => void;
  lockedCommand: CommandDefinition | null;
  initialValues: Record<string, unknown>;
  labelRanges: LabelRange[];
  currentParam: ParamDefinition | null;
  currentParamQuery: string;
  selectAutocomplete: (displayText: string, actualValue: string) => void;
  advanceParam: () => void;
  /** Desired cursor position after query update (null = end of string) */
  pendingCursorPos: number | null;
  clearPendingCursor: () => void;
}

// ── Tokenizer ──────────────────────────────────────────────────────────────────

interface Token {
  text: string;
  start: number;
  end: number;
}

interface TokenizeResult {
  tokens: Token[];
  trailingFragment: string;
  hasTrailingSpace: boolean;
}

function shellSplit(input: string): TokenizeResult {
  const tokens: Token[] = [];
  let current = "";
  let tokenStart = -1;
  let inQuote = false;
  let i = 0;

  while (i < input.length) {
    const ch = input[i];
    if (inQuote) {
      current += ch;
      if (ch === '"') inQuote = false;
    } else if (ch === '"') {
      if (tokenStart < 0) tokenStart = i;
      current += ch;
      inQuote = true;
    } else if (ch === " " || ch === "\t") {
      if (current.length > 0) {
        tokens.push({ text: current, start: tokenStart, end: i });
        current = "";
        tokenStart = -1;
      }
    } else {
      if (tokenStart < 0) tokenStart = i;
      current += ch;
    }
    i++;
  }

  const hasTrailingSpace =
    !inQuote && input.length > 0 && input[input.length - 1] === " ";

  if (current.length > 0 || inQuote) {
    return { tokens, trailingFragment: current, hasTrailingSpace: false };
  }

  return { tokens, trailingFragment: "", hasTrailingSpace };
}

function unquote(s: string): string {
  if (s.startsWith('"') && s.endsWith('"') && s.length >= 2)
    return s.slice(1, -1);
  return s;
}

// ── Arg parser ─────────────────────────────────────────────────────────────────

/** Returns true if a token is a label (e.g. "database:") or flag ("--priority") */
function isDelimiter(text: string): boolean {
  if (text.startsWith("--") && text.length > 2) return true;
  if (text.endsWith(":") && !text.startsWith("--")) return true;
  return false;
}

/**
 * Greedy parser: after a label, collects ALL subsequent tokens until the next
 * label/flag, joining them with spaces. This lets multi-word unquoted values
 * like `title: Testing Lemons` parse correctly as "Testing Lemons".
 */
function parseArgs(
  tokens: Token[],
  command: CommandDefinition,
): Record<string, unknown> {
  const requiredParams = command.params.filter((p) => p.group === "Required");
  const allParams = command.params;
  const values: Record<string, unknown> = {};
  let positionalIndex = 0;

  let i = 0;
  while (i < tokens.length) {
    const text = tokens[i].text;

    if (text.startsWith("--") && text.length > 2) {
      // Flag arg: --paramId value (single next token)
      const flagName = text.slice(2);
      const param = allParams.find(
        (p) => p.id.toLowerCase() === flagName.toLowerCase(),
      );
      if (param && i + 1 < tokens.length) {
        i++;
        values[param.id] = unquote(tokens[i].text);
      }
    } else if (text.endsWith(":") && !text.startsWith("--")) {
      // Inline label: greedily collect tokens until next label/flag
      const labelName = text.slice(0, -1).toLowerCase();
      const param = allParams.find(
        (p) => p.name.toLowerCase() === labelName,
      );
      if (param) {
        const parts: string[] = [];
        while (i + 1 < tokens.length && !isDelimiter(tokens[i + 1].text)) {
          i++;
          parts.push(unquote(tokens[i].text));
        }
        if (parts.length > 0) {
          values[param.id] = parts.join(" ");
        }
      }
    } else {
      // Bare positional arg
      if (positionalIndex < requiredParams.length) {
        values[requiredParams[positionalIndex].id] = unquote(text);
        positionalIndex++;
      }
    }
    i++;
  }

  return values;
}

/**
 * Like parseArgs but pads the input with a trailing space so shellSplit
 * finalizes the last token (no trailingFragment). Used by accept() and
 * advanceParam() to capture the full value including unfinished input.
 */
function parseArgsFull(
  argsStr: string,
  command: CommandDefinition,
): Record<string, unknown> {
  const padded = argsStr.endsWith(" ") ? argsStr : argsStr + " ";
  const { tokens } = shellSplit(padded);
  return parseArgs(tokens, command);
}

// ── Label range detection ──────────────────────────────────────────────────────

function computeLabelRanges(
  argsStr: string,
  argsOffset: number,
  command: CommandDefinition,
): LabelRange[] {
  const ranges: LabelRange[] = [];
  const params = command.params;
  const argsLower = argsStr.toLowerCase();

  for (const p of params) {
    const labelText = `${p.name.toLowerCase()}: `;
    let searchFrom = 0;
    while (searchFrom < argsLower.length) {
      const idx = argsLower.indexOf(labelText, searchFrom);
      if (idx < 0) break;
      if (idx === 0 || argsStr[idx - 1] === " ") {
        ranges.push({
          start: argsOffset + idx,
          end: argsOffset + idx + labelText.length,
        });
      }
      searchFrom = idx + 1;
    }
  }

  ranges.sort((a, b) => a.start - b.start);
  return ranges;
}

/** Find the next unfilled required param given current values. */
function findNextRequired(
  command: CommandDefinition,
  values: Record<string, unknown>,
): ParamDefinition | null {
  const requiredParams = command.params.filter((p) => p.group === "Required");
  return requiredParams.find((p) => values[p.id] === undefined) ?? null;
}

/** Returns true if a param accepts free-form text and benefits from auto-quotes. */
function needsQuotes(param: ParamDefinition): boolean {
  const t = param.param_type.type;
  return t === "Text" || t === "RichText";
}

/**
 * Append the next param label (and optional quotes) to a query string.
 * Returns { query, cursorPos } where cursorPos is between the quotes
 * for text params, or at the end of the label for picker params.
 */
function appendLabel(
  base: string,
  param: ParamDefinition,
): { query: string; cursorPos: number } {
  const withSpace = base.endsWith(" ") ? base : base + " ";
  const label = `${param.name.toLowerCase()}: `;
  if (needsQuotes(param)) {
    const query = withSpace + label + '""';
    return { query, cursorPos: query.length - 1 }; // between the quotes
  }
  const query = withSpace + label;
  return { query, cursorPos: query.length };
}

// ── Hook ───────────────────────────────────────────────────────────────────────

export function useCommandParser(
  rawQuery: string,
  setQuery: (q: string) => void,
): CommandParserState {
  const [commandSuggestions, setCommandSuggestions] = useState<
    CommandDefinition[]
  >([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [lockedCommand, setLockedCommand] = useState<CommandDefinition | null>(
    null,
  );
  const debounceRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const [pendingCursorPos, setPendingCursorPos] = useState<number | null>(null);
  const clearPendingCursor = useCallback(() => setPendingCursorPos(null), []);

  const input = rawQuery.startsWith("/") ? rawQuery.slice(1) : rawQuery;
  const phase = lockedCommand ? "args" : "selecting";

  // ── Phase: selecting ─────────────────────────────────────────────────────

  useEffect(() => {
    if (lockedCommand) return;
    if (!rawQuery.startsWith("/")) {
      setCommandSuggestions([]);
      return;
    }
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      const cmds = await searchCommands(input);
      setCommandSuggestions(cmds);
      setSelectedIndex(0);
    }, 80);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [rawQuery, input, lockedCommand]);

  // Unlock if user backtracks past the command prefix
  useEffect(() => {
    if (!lockedCommand) return;
    const prefix = `/${lockedCommand.extension_id} ${lockedCommand.name.toLowerCase()} `;
    if (!rawQuery.toLowerCase().startsWith(prefix.toLowerCase())) {
      setLockedCommand(null);
    }
  }, [rawQuery, lockedCommand]);

  // (No auto-inject on space — advancement is explicit via Tab or autocomplete)

  // ── Ghost text ───────────────────────────────────────────────────────────

  const ghostText = useMemo(() => {
    if (lockedCommand) {
      const prefix = `${lockedCommand.extension_id} ${lockedCommand.name.toLowerCase()} `;
      const argsStr = input.slice(prefix.length);
      const trimmedArgs = argsStr.trimEnd();

      // If args end with a pending label, show that param's placeholder
      if (trimmedArgs.endsWith(":") && argsStr.endsWith(" ")) {
        const { tokens } = shellSplit(trimmedArgs);
        const lastToken = tokens[tokens.length - 1];
        if (lastToken) {
          const labelName = lastToken.text.slice(0, -1).toLowerCase();
          const param = lockedCommand.params.find(
            (p) => p.name.toLowerCase() === labelName,
          );
          if (param) {
            return param.placeholder ?? param.name;
          }
        }
      }

      return "";
    }

    // Selecting phase
    const topCmd = commandSuggestions[selectedIndex];
    if (topCmd && input.length > 0) {
      const fullName = `${topCmd.extension_id} ${topCmd.name}`.toLowerCase();
      const inputLower = input.toLowerCase();
      if (fullName.startsWith(inputLower)) {
        return fullName.slice(input.length);
      }
    }
    return "";
  }, [lockedCommand, commandSuggestions, selectedIndex, input]);

  // ── Resolved values (display text → actual value, e.g. name → UUID) ────

  const resolvedValuesRef = useRef<Record<string, string>>({});

  // Reset resolved values when command changes
  useEffect(() => {
    resolvedValuesRef.current = {};
  }, [lockedCommand]);

  // ── Args parsing + label ranges ──────────────────────────────────────────

  const { initialValues, labelRanges } = useMemo(() => {
    if (!lockedCommand) {
      return {
        initialValues: {} as Record<string, unknown>,
        labelRanges: [] as LabelRange[],
      };
    }

    const prefix = `${lockedCommand.extension_id} ${lockedCommand.name.toLowerCase()} `;
    const argsStr = input.slice(prefix.length);
    const values = parseArgsFull(argsStr, lockedCommand);

    // Compute label ranges in the full rawQuery string
    const argsOffset = 1 + prefix.length; // +1 for leading /
    const ranges = computeLabelRanges(argsStr, argsOffset, lockedCommand);

    return { initialValues: values, labelRanges: ranges };
  }, [lockedCommand, input, rawQuery]);

  // ── Current param detection (which param is being typed right now) ──────

  const { currentParam, currentParamQuery } = useMemo((): {
    currentParam: ParamDefinition | null;
    currentParamQuery: string;
  } => {
    if (!lockedCommand || labelRanges.length === 0) {
      return { currentParam: null, currentParamQuery: "" };
    }

    // The last label range tells us which param is being filled
    const lastRange = labelRanges[labelRanges.length - 1];
    const labelText = rawQuery.slice(lastRange.start, lastRange.end);
    // labelText is like "database: " — extract name
    const labelName = labelText.slice(0, -2).trim().toLowerCase(); // remove ": "

    const param = lockedCommand.params.find(
      (p) => p.name.toLowerCase() === labelName,
    );
    if (!param) return { currentParam: null, currentParamQuery: "" };

    // Text after the last label = current param query
    const afterLabel = rawQuery.slice(lastRange.end);
    return { currentParam: param, currentParamQuery: afterLabel.trim() };
  }, [lockedCommand, labelRanges, rawQuery]);

  // ── Advance to next param (called by Tab / after autocomplete) ──────────

  const advanceParam = useCallback(() => {
    if (!lockedCommand) return;

    const prefix = `${lockedCommand.extension_id} ${lockedCommand.name.toLowerCase()} `;
    const argsStr = input.slice(prefix.length);
    const values = parseArgsFull(argsStr, lockedCommand);

    const next = findNextRequired(lockedCommand, values);
    if (!next) return; // All required params filled

    const { query: newQuery, cursorPos } = appendLabel(rawQuery, next);
    setPendingCursorPos(cursorPos);
    setQuery(newQuery);
  }, [lockedCommand, input, rawQuery, setQuery]);

  // ── Select autocomplete option ─────────────────────────────────────────

  const selectAutocomplete = useCallback(
    (displayText: string, actualValue: string) => {
      if (!lockedCommand || !currentParam) return;

      resolvedValuesRef.current[currentParam.id] = actualValue;

      // Find the last label range for the current param
      const lastRange = labelRanges[labelRanges.length - 1];
      if (!lastRange) return;

      const beforeValue = rawQuery.slice(0, lastRange.end);
      const quotedDisplay = displayText.includes(" ")
        ? `"${displayText}"`
        : displayText;

      // Build new args with selected value to find next unfilled param
      const prefix = `${lockedCommand.extension_id} ${lockedCommand.name.toLowerCase()} `;
      const newArgsStr = (
        beforeValue.slice(1) +
        quotedDisplay +
        " "
      ).slice(prefix.length);
      const values = parseArgsFull(newArgsStr, lockedCommand);
      // Mark current param filled (resolved value won't appear in parsed text)
      values[currentParam.id] = actualValue;

      const next = findNextRequired(lockedCommand, values);
      if (next) {
        const base = beforeValue + quotedDisplay;
        const { query: newQuery, cursorPos } = appendLabel(base, next);
        setPendingCursorPos(cursorPos);
        setQuery(newQuery);
      } else {
        setQuery(beforeValue + quotedDisplay + " ");
      }
    },
    [lockedCommand, currentParam, labelRanges, rawQuery, setQuery],
  );

  // ── Accept ───────────────────────────────────────────────────────────────

  const accept = useCallback((): {
    command: CommandDefinition;
    initialValues: Record<string, unknown>;
  } | null => {
    if (lockedCommand) {
      // Parse with full capture (including trailing text via padding)
      const prefix = `${lockedCommand.extension_id} ${lockedCommand.name.toLowerCase()} `;
      const argsStr = input.slice(prefix.length);
      const values = parseArgsFull(argsStr, lockedCommand);

      // Merge resolved values (UUIDs) over display text values
      for (const [key, val] of Object.entries(resolvedValuesRef.current)) {
        if (val) values[key] = val;
      }
      return { command: lockedCommand, initialValues: values };
    }

    // Selecting phase — lock the selected command
    const cmd = commandSuggestions[selectedIndex] ?? null;
    if (!cmd) return null;

    // Check if there are already args typed after the command name
    const fullName = `${cmd.extension_id} ${cmd.name}`.toLowerCase();
    const inputLower = input.toLowerCase();

    if (inputLower.length > fullName.length + 1) {
      // User typed beyond the command name — parse args immediately
      const prefix = `${cmd.extension_id} ${cmd.name.toLowerCase()} `;
      const argsStr = input.slice(prefix.length);
      const values = parseArgsFull(argsStr, cmd);
      setLockedCommand(cmd);
      return { command: cmd, initialValues: values };
    }

    // Lock command and inject first required label
    const firstRequired = cmd.params.find((p) => p.group === "Required");
    const cmdBase = `/${cmd.extension_id} ${cmd.name.toLowerCase()}`;
    if (firstRequired) {
      const { query: newQuery, cursorPos } = appendLabel(cmdBase, firstRequired);
      setPendingCursorPos(cursorPos);
      setLockedCommand(cmd);
      setQuery(newQuery);
    } else {
      setLockedCommand(cmd);
      setQuery(cmdBase + " ");
    }
    return null;
  }, [
    commandSuggestions,
    selectedIndex,
    lockedCommand,
    initialValues,
    input,
    setQuery,
  ]);

  return {
    phase,
    commandSuggestions,
    selectedIndex,
    ghostText,
    accept,
    setSelectedIndex,
    lockedCommand,
    initialValues,
    labelRanges,
    currentParam,
    currentParamQuery,
    selectAutocomplete,
    advanceParam,
    pendingCursorPos,
    clearPendingCursor,
  };
}
