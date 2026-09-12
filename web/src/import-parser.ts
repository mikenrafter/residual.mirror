// Parser for pasted `residual add <type> ...` / `residual update <type> ...`
// command-line text into typed pending items, for the web import UI.
//
// Flag validity is sourced from web/generated/cli-schema.json (produced by
// `residual internal cli-schema`), not hand-duplicated here — see
// import-parser.parity.test.ts.

/** A single pending mutation parsed from one line of import text. */
export interface PendingItem {
  /** "add" or "update" — mirrors the residual CLI verb. */
  kind: "add" | "update";
  /** The force/entity type, e.g. "stressor", "component", "term". */
  type: string;
  /**
   * All recognized flags for this line, keyed by kebab-case flag name
   * (matching the real `--flag-name` CLI syntax) with their string value.
   * Flags declared `multiple: true` in the CLI schema are NOT stored here;
   * they appear in `multipleFields` instead.
   */
  fields: Record<string, string>;
  /**
   * Values for flags declared `multiple: true` in the CLI schema (e.g.
   * `--add-component`/`--remove-component` on `update stressor` /
   * `update purpose`), keyed by kebab-case flag name, collecting every
   * occurrence in the order they appeared on the line.
   */
  multipleFields: Record<string, string[]>;
  /** The original source line, verbatim, for display/debugging. */
  raw: string;
}

/** A line that failed to parse into a PendingItem, with a human-readable reason. */
export interface ParseError {
  /** The original source line, verbatim. */
  line: string;
  /** Human-readable description of why the line could not be parsed. */
  message: string;
}

/** Result of parsing a full block of pasted import text. */
export interface ParseResult {
  items: PendingItem[];
  errors: ParseError[];
}

import cliSchema from "../generated/cli-schema.json";

/** Flag metadata as recognized internally by the parser, for one subcommand. */
export interface KnownFlag {
  name: string;
  required: boolean;
  multiple: boolean;
}

interface SchemaFlag {
  name: string;
  required: boolean;
  multiple: boolean;
}

interface SchemaEntry {
  subcommand: string;
  flags: SchemaFlag[];
}

const schema = cliSchema as SchemaEntry[];

const knownFlagsBySubcommand = new Map<string, KnownFlag[]>(
  schema.map((entry) => [
    entry.subcommand,
    entry.flags.map((flag) => ({
      name: flag.name,
      required: flag.required,
      multiple: flag.multiple,
    })),
  ]),
);

/**
 * Returns the parser's internal record of known flags for a given
 * "add <type>" / "update <type>" subcommand string (e.g. "update stressor"),
 * or undefined if the subcommand is not recognized at all.
 *
 * This is the table the parity test walks `cli-schema.json` against: for
 * every `{ subcommand, flags }` entry there, every flag name must appear
 * here with matching `required`/`multiple`.
 */
export function getKnownFlags(subcommand: string): KnownFlag[] | undefined {
  return knownFlagsBySubcommand.get(subcommand);
}

/**
 * Tokenizes a single command line into shell-style words, honoring
 * double-quoted segments (`--description "some text"` -> one token
 * `some text`) and `\"` escapes within a quoted segment.
 *
 * Exported for reuse by the parity test and for direct unit testing of
 * quoting edge cases.
 */
export function tokenizeCommandLine(line: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let inQuotes = false;
  let hasToken = false;

  for (let i = 0; i < line.length; i++) {
    const char = line[i];

    if (inQuotes) {
      if (char === "\\" && line[i + 1] === '"') {
        current += '"';
        i++;
      } else if (char === '"') {
        inQuotes = false;
      } else {
        current += char;
      }
      continue;
    }

    if (char === '"') {
      inQuotes = true;
      hasToken = true;
      continue;
    }

    if (char === " " || char === "\t") {
      if (hasToken) {
        tokens.push(current);
        current = "";
        hasToken = false;
      }
      continue;
    }

    current += char;
    hasToken = true;
  }

  if (hasToken) {
    tokens.push(current);
  }

  return tokens;
}

/**
 * Parses multi-line pasted `residual add ...` / `residual update ...`
 * command text into PendingItems.
 *
 * - Blank lines and lines starting with `#` are skipped (not errors).
 * - Each remaining line must be a `residual add <type> ...` or
 *   `residual update <type> ...` invocation with recognized flags for
 *   that subcommand; anything else produces a ParseError for that line
 *   rather than throwing.
 */
export function parseImportText(text: string): ParseResult {
  const items: PendingItem[] = [];
  const errors: ParseError[] = [];

  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) {
      continue;
    }

    const result = parseLine(trimmed);
    if ("item" in result) {
      items.push(result.item);
    } else {
      errors.push({ line: trimmed, message: result.message });
    }
  }

  return { items, errors };
}

function parseLine(
  line: string,
): { item: PendingItem } | { message: string } {
  const tokens = tokenizeCommandLine(line);

  if (tokens[0] !== "residual") {
    return { message: "not a residual add/update command" };
  }

  const verb = tokens[1];
  if (verb !== "add" && verb !== "update") {
    return { message: "not a residual add/update command" };
  }

  const type = tokens[2];
  if (type === undefined) {
    return { message: "not a residual add/update command" };
  }

  const subcommand = `${verb} ${type}`;
  const knownFlags = getKnownFlags(subcommand);
  if (knownFlags === undefined) {
    return { message: `unrecognized subcommand "${subcommand}"` };
  }

  const knownFlagsByName = new Map(knownFlags.map((f) => [f.name, f]));
  const fields: Record<string, string> = {};
  const multipleFields: Record<string, string[]> = {};

  let i = 3;
  while (i < tokens.length) {
    const flagToken = tokens[i]!;
    if (!flagToken.startsWith("--")) {
      return { message: `unexpected token "${flagToken}"` };
    }
    const flagName = flagToken.slice(2);
    const known = knownFlagsByName.get(flagName);
    if (known === undefined) {
      return { message: `unrecognized flag "--${flagName}"` };
    }

    const value = tokens[i + 1];
    if (value === undefined) {
      return { message: `missing value for flag "--${flagName}"` };
    }

    if (known.multiple) {
      const existing = multipleFields[flagName];
      if (existing) {
        existing.push(value);
      } else {
        multipleFields[flagName] = [value];
      }
    } else {
      fields[flagName] = value;
    }

    i += 2;
  }

  const missing = knownFlags.filter((flag) => {
    if (!flag.required) return false;
    if (flag.multiple) {
      return (multipleFields[flag.name]?.length ?? 0) === 0;
    }
    return fields[flag.name] === undefined;
  });

  if (missing.length > 0) {
    const names = missing.map((f) => `--${f.name}`).join(", ");
    return { message: `missing required flag(s): ${names}` };
  }

  return {
    item: {
      kind: verb,
      type,
      fields,
      multipleFields,
      raw: line,
    },
  };
}
