import { describe, expect, it } from "vitest";

import {
  csvToJson,
  formatJson,
  jsonToCsv,
  MAX_TEXT_INPUT,
  minifyCss,
  minifyHtml,
  minifyJson,
  processTextUtility,
  slugify,
  unicodeWordCount,
} from "./utility-processors";

describe("utility processors", () => {
  it("handles Unicode text without corrupting scripts", () => {
    const mixed = "Rakyat سلام 中文 한국어";
    expect(unicodeWordCount(mixed)).toBe(4);
    expect(processTextUtility("text-case-converter", mixed, "upper").output).toContain("中文");
    expect(slugify("Café Rakyat 中文")).toBe("cafe-rakyat-中文");
  });

  it("formats, validates, and minifies JSON", () => {
    expect(formatJson('{"a":1}')).toContain("\n");
    expect(minifyJson('{ "a": 1 }')).toBe('{"a":1}');
    expect(processTextUtility("dev-json-validator", "[]").output).toBe("Valid JSON");
    expect(() => minifyJson("{")) .toThrow();
  });

  it("round-trips representative CSV and JSON including quoted commas", () => {
    const json = csvToJson('name,note\nAli,"one, two"');
    expect(JSON.parse(json)).toEqual([{ name: "Ali", note: "one, two" }]);
    expect(jsonToCsv(json)).toBe('name,note\nAli,"one, two"');
  });

  it("minifies CSS and HTML", () => {
    expect(minifyCss("/* x */ .a { color: red; }")).toBe(".a{color:red}");
    expect(minifyHtml("<!-- x --><p>  Hello </p>\n <p>World</p>")).toBe("<p> Hello </p><p>World</p>");
  });

  it("computes representative calculator outputs", () => {
    expect(processTextUtility("calc-discount", "100,20").output).toContain("80.00");
    expect(processTextUtility("calc-bmi", "70,175").output).toContain("22.86");
  });

  it("refuses oversized text", () => {
    expect(() => slugify("x".repeat(MAX_TEXT_INPUT + 1))).toThrow(/character limit/);
  });
});
