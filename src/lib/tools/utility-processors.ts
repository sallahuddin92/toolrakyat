export const MAX_TEXT_INPUT = 1_000_000;

export type UtilityTextResult = {
  output: string;
  mime?: string;
  extension?: string;
};

function bounded(input: string): string {
  if (input.length > MAX_TEXT_INPUT) {
    throw new Error(`Input exceeds the ${MAX_TEXT_INPUT.toLocaleString()} character limit.`);
  }
  return input;
}

function unicodeWords(input: string): string[] {
  return input.trim().match(/[\p{L}\p{N}]+(?:['’_-][\p{L}\p{N}]+)*/gu) ?? [];
}

function titleCase(input: string): string {
  return input.toLocaleLowerCase().replace(/(^|[^\p{L}\p{N}])(\p{L})/gu, (_, prefix, letter) =>
    `${prefix}${letter.toLocaleUpperCase()}`
  );
}

function sentenceCase(input: string): string {
  return input.toLocaleLowerCase().replace(/(^|[.!?]\s+)(\p{L})/gu, (_, prefix, letter) =>
    `${prefix}${letter.toLocaleUpperCase()}`
  );
}

function parseNumbers(input: string, count: number): number[] {
  const values = input.split(/[\s,;]+/).filter(Boolean).map(Number);
  if (values.length < count || values.slice(0, count).some((value) => !Number.isFinite(value))) {
    throw new Error(`Enter at least ${count} valid numbers separated by commas.`);
  }
  return values;
}

function csvEscape(value: unknown): string {
  const text = value == null ? "" : String(value);
  return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

export function slugify(input: string): string {
  return bounded(input)
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase()
    .trim()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "");
}

export function minifyJson(input: string): string {
  return JSON.stringify(JSON.parse(bounded(input)));
}

export function formatJson(input: string): string {
  return JSON.stringify(JSON.parse(bounded(input)), null, 2);
}

export function validateJson(input: string): string {
  JSON.parse(bounded(input));
  return "Valid JSON";
}

export function minifyCss(input: string): string {
  return bounded(input)
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/\s+/g, " ")
    .replace(/\s*([{}:;,>])\s*/g, "$1")
    .replace(/;}/g, "}")
    .trim();
}

export function minifyHtml(input: string): string {
  return bounded(input)
    .replace(/<!--(?!\[if)[\s\S]*?-->/g, "")
    .replace(/>\s+</g, "><")
    .replace(/\s{2,}/g, " ")
    .trim();
}

export function csvToJson(input: string): string {
  const lines = bounded(input).replace(/^\uFEFF/, "").split(/\r?\n/).filter((line) => line.trim());
  if (lines.length < 2) throw new Error("CSV must contain a header and at least one row.");
  const parse = (line: string) => {
    const cells: string[] = [];
    let cell = "";
    let quoted = false;
    for (let index = 0; index < line.length; index += 1) {
      const char = line[index];
      if (char === '"' && quoted && line[index + 1] === '"') {
        cell += '"';
        index += 1;
      } else if (char === '"') quoted = !quoted;
      else if (char === "," && !quoted) {
        cells.push(cell);
        cell = "";
      } else cell += char;
    }
    cells.push(cell);
    if (quoted) throw new Error("CSV contains an unterminated quoted field.");
    return cells;
  };
  const headers = parse(lines[0]);
  const rows = lines.slice(1).map((line) => {
    const values = parse(line);
    return Object.fromEntries(headers.map((header, index) => [header, values[index] ?? ""]));
  });
  return JSON.stringify(rows, null, 2);
}

export function jsonToCsv(input: string): string {
  const value: unknown = JSON.parse(bounded(input));
  if (!Array.isArray(value) || value.length === 0 || value.some((row) => !row || typeof row !== "object" || Array.isArray(row))) {
    throw new Error("JSON must be a non-empty array of objects.");
  }
  const rows = value as Record<string, unknown>[];
  const headers = [...new Set(rows.flatMap(Object.keys))];
  return [headers.map(csvEscape).join(","), ...rows.map((row) => headers.map((key) => csvEscape(row[key])).join(","))].join("\n");
}

export function processTextUtility(toolId: string, rawInput: string, option = ""): UtilityTextResult {
  const input = bounded(rawInput);
  switch (toolId) {
    case "text-slug-generator": return { output: slugify(input) };
    case "text-case-converter": {
      const mode = option || "upper";
      const output = mode === "lower" ? input.toLocaleLowerCase() : mode === "title" ? titleCase(input) : mode === "sentence" ? sentenceCase(input) : input.toLocaleUpperCase();
      return { output };
    }
    case "text-cleaner": return { output: input.replace(/[ \t]+$/gm, "").replace(/\n{3,}/g, "\n\n").trim() };
    case "compression-json-minifier": return { output: minifyJson(input), mime: "application/json", extension: "json" };
    case "compression-css-minifier": return { output: minifyCss(input), mime: "text/css", extension: "css" };
    case "compression-html-minifier": return { output: minifyHtml(input), mime: "text/html", extension: "html" };
    case "converter-csv-to-json": return { output: csvToJson(input), mime: "application/json", extension: "json" };
    case "converter-json-to-csv": return { output: jsonToCsv(input), mime: "text/csv", extension: "csv" };
    case "dev-json-formatter": return { output: formatJson(input), mime: "application/json", extension: "json" };
    case "dev-json-validator": return { output: validateJson(input) };
    case "dev-url-encode": return { output: option === "decode" ? decodeURIComponent(input) : encodeURIComponent(input) };
    case "dev-base64": {
      if (option === "decode") {
        const bytes = Uint8Array.from(atob(input), (char) => char.charCodeAt(0));
        return { output: new TextDecoder().decode(bytes) };
      }
      const bytes = new TextEncoder().encode(input);
      let binary = "";
      bytes.forEach((byte) => { binary += String.fromCharCode(byte); });
      return { output: btoa(binary) };
    }
    case "calc-loan": {
      const [principal, annualRate, years] = parseNumbers(input, 3);
      const months = Math.round(years * 12);
      const rate = annualRate / 1200;
      const payment = rate === 0 ? principal / months : principal * rate * (1 + rate) ** months / ((1 + rate) ** months - 1);
      return { output: `Monthly payment: ${payment.toFixed(2)}\nTotal payment: ${(payment * months).toFixed(2)}` };
    }
    case "calc-profit-margin": {
      const [cost, revenue] = parseNumbers(input, 2);
      if (revenue === 0) throw new Error("Revenue must be greater than zero.");
      return { output: `Profit: ${(revenue - cost).toFixed(2)}\nMargin: ${(((revenue - cost) / revenue) * 100).toFixed(2)}%` };
    }
    case "calc-discount": {
      const [price, percent] = parseNumbers(input, 2);
      return { output: `Discount: ${(price * percent / 100).toFixed(2)}\nFinal price: ${(price * (1 - percent / 100)).toFixed(2)}` };
    }
    case "calc-sst": {
      const [amount, percent = 8] = parseNumbers(input, 1);
      return { output: `SST: ${(amount * percent / 100).toFixed(2)}\nTotal: ${(amount * (1 + percent / 100)).toFixed(2)}` };
    }
    case "calc-bmi": {
      const [weightKg, heightCm] = parseNumbers(input, 2);
      const bmi = weightKg / (heightCm / 100) ** 2;
      return { output: `BMI: ${bmi.toFixed(2)}` };
    }
    case "calc-compound-interest": {
      const [principal, annualRate, years, compounds = 12] = parseNumbers(input, 3);
      const total = principal * (1 + annualRate / 100 / compounds) ** (compounds * years);
      return { output: `Final amount: ${total.toFixed(2)}\nInterest earned: ${(total - principal).toFixed(2)}` };
    }
    case "calc-age": {
      const birth = new Date(input);
      if (Number.isNaN(birth.getTime()) || birth > new Date()) throw new Error("Enter a valid past date (YYYY-MM-DD).");
      const now = new Date();
      let years = now.getFullYear() - birth.getFullYear();
      if (now < new Date(now.getFullYear(), birth.getMonth(), birth.getDate())) years -= 1;
      return { output: `Age: ${years} years` };
    }
    case "calc-date-difference": {
      const [startRaw, endRaw] = input.split(/[\s,;]+/);
      const start = new Date(startRaw); const end = new Date(endRaw);
      if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) throw new Error("Enter two dates (YYYY-MM-DD YYYY-MM-DD).");
      return { output: `Difference: ${Math.abs(Math.round((end.getTime() - start.getTime()) / 86_400_000))} days` };
    }
    case "dev-color-converter": {
      const hex = input.trim().replace(/^#/, "");
      if (!/^[0-9a-f]{6}$/i.test(hex)) throw new Error("Enter a six-digit hex colour.");
      const [r, g, b] = [0, 2, 4].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16));
      return { output: `HEX: #${hex.toUpperCase()}\nRGB: rgb(${r}, ${g}, ${b})` };
    }
    default: return { output: input };
  }
}

export function generatePassword(length = 20): string {
  if (!Number.isInteger(length) || length < 8 || length > 128) throw new Error("Password length must be 8–128.");
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*";
  const bytes = crypto.getRandomValues(new Uint8Array(length));
  return Array.from(bytes, (byte) => alphabet[byte % alphabet.length]).join("");
}

export function unicodeWordCount(input: string): number {
  return unicodeWords(bounded(input)).length;
}
