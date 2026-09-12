import { describe, expect, test } from "bun:test";
import { parseImportText } from "./import-parser";

describe("parseImportText", () => {
  test("skips blank lines and comment lines without producing errors or items", () => {
    const text = [
      "",
      "# this is a comment",
      "   ",
      "# residual add stressor --description x --attractor-id A-01 --naive-change y",
    ].join("\n");

    const result = parseImportText(text);

    expect(result.errors).toHaveLength(0);
    expect(result.items).toHaveLength(0);
  });

  test("parses a basic `residual add stressor` line into a PendingItem", () => {
    const line =
      'residual add stressor --description "db migration fails midway" --attractor-id A-01 --naive-change "rollback script"';

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    expect(result.items).toHaveLength(1);
    const item = result.items[0]!;
    expect(item.kind).toBe("add");
    expect(item.type).toBe("stressor");
    expect(item.fields.description).toBe("db migration fails midway");
    expect(item.fields["attractor-id"]).toBe("A-01");
    expect(item.fields["naive-change"]).toBe("rollback script");
  });

  test("parses a basic `residual update stressor` line into a PendingItem", () => {
    const line = 'residual update stressor --force-id S-01 --description "revised text"';

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    expect(result.items).toHaveLength(1);
    const item = result.items[0]!;
    expect(item.kind).toBe("update");
    expect(item.type).toBe("stressor");
    expect(item.fields["force-id"]).toBe("S-01");
    expect(item.fields.description).toBe("revised text");
  });

  test("handles double-quoted values with embedded spaces", () => {
    const line =
      'residual add term --term "hyperliminal coupling" --definition "a coupling that crosses architectural boundaries invisibly"';

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    const item = result.items[0]!;
    expect(item.fields.term).toBe("hyperliminal coupling");
    expect(item.fields.definition).toBe(
      "a coupling that crosses architectural boundaries invisibly",
    );
  });

  test("handles escaped double quotes inside a quoted value", () => {
    const line = 'residual add term --term foo --definition "a value with \\"nested\\" quotes"';

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    const item = result.items[0]!;
    expect(item.fields.definition).toBe('a value with "nested" quotes');
  });

  test("collects repeated `multiple: true` flags into an array on update stressor", () => {
    const line =
      "residual update stressor --force-id S-01 --add-component cli --add-component storage";

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    const item = result.items[0]!;
    expect(item.multipleFields["add-component"]).toEqual(["cli", "storage"]);
  });

  test("collects repeated `remove-component` flags on update purpose", () => {
    const line =
      "residual update purpose --force-id P-01 --remove-component web --remove-component cli";

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    const item = result.items[0]!;
    expect(item.multipleFields["remove-component"]).toEqual(["web", "cli"]);
  });

  test("supports both add-component and remove-component together", () => {
    const line =
      "residual update stressor --force-id S-01 --add-component cli --remove-component storage";

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    const item = result.items[0]!;
    expect(item.multipleFields["add-component"]).toEqual(["cli"]);
    expect(item.multipleFields["remove-component"]).toEqual(["storage"]);
  });

  test("produces a ParseError when a required flag is missing (add stressor missing --description)", () => {
    const line = "residual add stressor --attractor-id A-01 --naive-change y";

    const result = parseImportText(line);

    expect(result.items).toHaveLength(0);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]!.line).toBe(line);
    expect(result.errors[0]!.message.toLowerCase()).toContain("description");
  });

  test("produces a ParseError when update stressor is missing required --force-id", () => {
    const line = 'residual update stressor --description "revised text"';

    const result = parseImportText(line);

    expect(result.items).toHaveLength(0);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]!.message.toLowerCase()).toContain("force-id");
  });

  test("produces a ParseError for an unrecognized flag on a known subcommand", () => {
    const line =
      'residual add stressor --description x --attractor-id A-01 --naive-change y --bogus-flag z';

    const result = parseImportText(line);

    expect(result.items).toHaveLength(0);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]!.message).toContain("bogus-flag");
  });

  test("produces a ParseError for an unrecognized subcommand type", () => {
    const line = "residual add nonsense --description x";

    const result = parseImportText(line);

    expect(result.items).toHaveLength(0);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]!.line).toBe(line);
  });

  test("produces a ParseError for a line that isn't a residual add/update invocation", () => {
    const line = "git status --short";

    const result = parseImportText(line);

    expect(result.items).toHaveLength(0);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]!.message.toLowerCase()).toContain(
      "not a residual add/update command",
    );
  });

  test("produces a ParseError for a bare `residual` with no verb", () => {
    const line = "residual";

    const result = parseImportText(line);

    expect(result.items).toHaveLength(0);
    expect(result.errors).toHaveLength(1);
  });

  test("parses multiple lines independently, collecting both items and errors", () => {
    const text = [
      'residual add stressor --description x --attractor-id A-01 --naive-change y',
      "not a residual command at all",
      'residual update stressor --force-id S-01 --add-component cli',
    ].join("\n");

    const result = parseImportText(text);

    expect(result.items).toHaveLength(2);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]!.line).toBe("not a residual command at all");
  });

  test("parses `residual add component` with all required flags", () => {
    const line =
      'residual add component --name gateway --description "edge api gateway" --status proposed --architecture-set iter1';

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    const item = result.items[0]!;
    expect(item.type).toBe("component");
    expect(item.fields.name).toBe("gateway");
    expect(item.fields.status).toBe("proposed");
    expect(item.fields["architecture-set"]).toBe("iter1");
  });

  test("optional flags are omitted from fields when not provided", () => {
    const line =
      'residual add stressor --description x --attractor-id A-01 --naive-change y';

    const result = parseImportText(line);

    expect(result.errors).toHaveLength(0);
    const item = result.items[0]!;
    expect(item.fields.notes).toBeUndefined();
    expect(item.fields.shortname).toBeUndefined();
  });
});
