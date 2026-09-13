import { describe, expect, test } from "bun:test";
import cliSchema from "../generated/cli-schema.json";
import { getKnownFlags } from "./import-parser";

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

describe("import-parser / cli-schema.json parity", () => {
  test("cli-schema.json is loaded and non-empty (sanity check on the fixture itself)", () => {
    expect(Array.isArray(schema)).toBe(true);
    expect(schema.length).toBeGreaterThan(0);
  });

  for (const entry of schema) {
    test(`parser recognizes every documented flag for "${entry.subcommand}"`, () => {
      const known = getKnownFlags(entry.subcommand);

      expect(known).toBeDefined();

      const byName = new Map((known ?? []).map((f) => [f.name, f]));

      for (const flag of entry.flags) {
        const match = byName.get(flag.name);
        expect(
          match,
          `expected parser to know about --${flag.name} for "${entry.subcommand}"`,
        ).toBeDefined();
        expect(match?.required).toBe(flag.required);
        expect(match?.multiple).toBe(flag.multiple);
      }
    });
  }

  test("every subcommand in cli-schema.json is known to the parser (no missing subcommands)", () => {
    const missing = schema
      .map((entry) => entry.subcommand)
      .filter((subcommand) => getKnownFlags(subcommand) === undefined);

    expect(missing).toEqual([]);
  });
});
