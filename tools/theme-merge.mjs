#!/usr/bin/env node
/**
 * 把 intellij-community 的主题继承链合并成「键 → 最终值 + 来源」清单。
 *
 * 只读：不修改 intellij-community 里的任何文件，也不修改 themes/*.json。
 *
 * 用法：
 *   node tools/theme-merge.mjs
 *   node tools/theme-merge.mjs --root D:\Programming\Workspace\intellij-community
 *   node tools/theme-merge.mjs --themes "Islands Light,Islands Dark" --out themes/generated
 *
 * 输出（--out 目录下）：
 *   <slug>.resolved.json    每个键的最终值、别名链、来源文件:行
 *   <slug>.unresolved.json  无法解析的键（父主题缺失 / 只能靠代码默认值兜底）
 *   keel-diff-<slug>.md     与 themes/idea-light.json 的差异清单
 *
 * 设计取舍：
 *   1. 主题 JSON 允许注释、并且同一个文件里既有嵌套对象也有字面量点号键，
 *      所以这里自带一个带行号的 JSON 解析器，而不是 JSON.parse。
 *   2. `os.windows` / `os.default` 这样的平台覆盖：按 Windows 取值，
 *      并在报告里保留原始值，方便核对。
 *   3. `transparent` 映射为 `#00000000`（IDEA 主题里的关键字），报告里标注 assumption。
 */

import { readFileSync, writeFileSync, mkdirSync, readdirSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

// ---------------------------------------------------------------------------
// 命令行参数
// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg.startsWith("--")) out[arg.slice(2)] = argv[++i];
  }
  return out;
}

const args = parseArgs(process.argv.slice(2));
const ROOT = resolve(
  args.root ?? "D:/Programming/Workspace/intellij-community",
);
const SRC = join(ROOT, "platform", "platform-resources", "src");
const OUT = resolve(args.out ?? "themes/generated");
const THEMES = (args.themes ?? "Islands Light,Islands Dark")
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);

if (!existsSync(SRC)) {
  console.error(`找不到 platform-resources：${SRC}`);
  process.exit(2);
}

// ---------------------------------------------------------------------------
// 带行号的 JSON 解析（支持 // 与 /* */ 注释、字符串转义）
// ---------------------------------------------------------------------------

function parseJsonWithLines(text) {
  let i = 0;
  let line = 1;
  /** @type {Map<string, number>} 路径（用 \u0000 连接）→ 行号 */
  const lines = new Map();

  const pathKey = (path) => path.join("\u0000");

  function skipTrivia() {
    for (;;) {
      const c = text[i];
      if (c === "\n") {
        line++;
        i++;
      } else if (c === " " || c === "\t" || c === "\r") {
        i++;
      } else if (c === "/" && text[i + 1] === "/") {
        while (i < text.length && text[i] !== "\n") i++;
      } else if (c === "/" && text[i + 1] === "*") {
        i += 2;
        while (i < text.length && !(text[i] === "*" && text[i + 1] === "/")) {
          if (text[i] === "\n") line++;
          i++;
        }
        i += 2;
      } else {
        return;
      }
    }
  }

  function parseString() {
    // 调用时 text[i] 一定是 '"'
    i++;
    let out = "";
    while (i < text.length) {
      const c = text[i];
      if (c === '"') {
        i++;
        return out;
      }
      if (c === "\\") {
        const esc = text[i + 1];
        i += 2;
        if (esc === "u") {
          out += String.fromCharCode(parseInt(text.slice(i, i + 4), 16));
          i += 4;
        } else {
          const map = { n: "\n", r: "\r", t: "\t", b: "\b", f: "\f", '"': '"', "\\": "\\", "/": "/" };
          out += map[esc] ?? esc;
        }
      } else {
        if (c === "\n") line++;
        out += c;
        i++;
      }
    }
    throw new Error("字符串没有闭合");
  }

  function parseValue(path) {
    skipTrivia();
    const c = text[i];
    if (c === "{") return parseObject(path);
    if (c === "[") return parseArray(path);
    if (c === '"') return parseString();
    // 数字 / true / false / null
    const start = i;
    while (i < text.length && !",}] \t\r\n/".includes(text[i])) i++;
    const raw = text.slice(start, i);
    if (raw === "true") return true;
    if (raw === "false") return false;
    if (raw === "null") return null;
    const n = Number(raw);
    if (Number.isNaN(n)) throw new Error(`无法解析的值：${raw}`);
    return n;
  }

  function parseObject(path) {
    const obj = {};
    i++; // {
    for (;;) {
      skipTrivia();
      if (text[i] === "}") {
        i++;
        return obj;
      }
      if (text[i] !== '"') throw new Error(`期望对象键，实际是 ${text[i]}`);
      const keyLine = line;
      const key = parseString();
      lines.set(pathKey([...path, key]), keyLine);
      skipTrivia();
      if (text[i] !== ":") throw new Error(`期望 ':'，实际是 ${text[i]}`);
      i++;
      obj[key] = parseValue([...path, key]);
      skipTrivia();
      if (text[i] === ",") {
        i++;
        continue;
      }
      if (text[i] === "}") {
        i++;
        return obj;
      }
      throw new Error(`期望 ',' 或 '}'，实际是 ${text[i]}`);
    }
  }

  function parseArray(path) {
    const arr = [];
    i++; // [
    let index = 0;
    for (;;) {
      skipTrivia();
      if (text[i] === "]") {
        i++;
        return arr;
      }
      arr.push(parseValue([...path, String(index++)]));
      skipTrivia();
      if (text[i] === ",") {
        i++;
        continue;
      }
      if (text[i] === "]") {
        i++;
        return arr;
      }
      throw new Error(`期望 ',' 或 ']'，实际是 ${text[i]}`);
    }
  }

  const value = parseValue([]);
  return { value, lines };
}

// ---------------------------------------------------------------------------
// 主题注册表（从 META-INF 的 <themeProvider> 读，避免硬编码路径）
// ---------------------------------------------------------------------------

const SKIP_DIRS = new Set([
  ".git",
  "build",
  "out",
  "gen",
  "node_modules",
  "test",
  "tests",
  "testData",
]);

function* walkXml(dir, depth = 0) {
  if (depth > 12) return;
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      yield* walkXml(full, depth + 1);
    } else if (entry.name.endsWith(".xml")) {
      yield full;
    }
  }
}

/**
 * 从所有声明 <themeProvider> 的 XML 里读主题 id → 路径。
 * ThemeProvider 实际声明在 platform-impl/resources，不在 platform-resources/META-INF，
 * 所以这里扫描整个 platform 目录（跳过测试与构建产物）。
 */
function collectProviders() {
  const map = new Map();
  for (const file of walkXml(join(ROOT, "platform"))) {
    let xml;
    try {
      xml = readFileSync(file, "utf8");
    } catch {
      continue;
    }
    if (!xml.includes("themeProvider")) continue;
    const re = /<themeProvider\b[^>]*>/g;
    let match;
    while ((match = re.exec(xml))) {
      const tag = match[0];
      const id = /\bid="([^"]+)"/.exec(tag)?.[1];
      const path = /\bpath="([^"]+)"/.exec(tag)?.[1];
      if (id && path) map.set(id, path.replace(/^\//, ""));
    }
  }
  return map;
}

const providers = collectProviders();

function themeFile(idOrName) {
  const rel = providers.get(idOrName);
  if (rel) return join(SRC, rel);
  // 兜底：同名文件
  for (const dir of ["themes", "themes/expUI", "themes/islands"]) {
    const p = join(SRC, dir, `${idOrName}.theme.json`);
    if (existsSync(p)) return p;
  }
  return null;
}

function loadTheme(idOrName) {
  const file = themeFile(idOrName);
  if (!file) throw new Error(`找不到主题 ${idOrName}`);
  const { value, lines } = parseJsonWithLines(readFileSync(file, "utf8"));
  return { id: idOrName, file, data: value, lines };
}

function buildChain(idOrName) {
  const chain = [];
  let current = idOrName;
  for (let depth = 0; depth < 10; depth++) {
    const theme = loadTheme(current);
    chain.unshift(theme);
    const parent = theme.data.parentTheme;
    if (!parent) break;
    current = parent;
  }
  return chain;
}

// ---------------------------------------------------------------------------
// 合并（父 → 子），记录每个键的来源
// ---------------------------------------------------------------------------

const CLASS_NAME = /^[a-zA-Z_$][\w$]*(\.[a-zA-Z_$][\w$]*)+$/;
const ICON_PATH = /\.svg$/i;

function isColorish(value) {
  return typeof value === "string" && /^#([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(value);
}

function mergeTheme(chain) {
  const palette = new Map(); // name -> { value, source }
  const tokens = new Map(); // key -> { value, source, platform }
  const icons = new Map();
  const notes = [];

  for (const theme of chain) {
    const relFile = theme.file.slice(SRC.length + 1).replace(/\\/g, "/");
    const { data, lines } = theme;

    const lineOf = (path) =>
      lines.get(["ui", ...path].join("\u0000")) ?? lines.get(path.join("\u0000")) ?? 0;
    const source = (path) => `${relFile}:${lineOf(path)}`;

    for (const [name, value] of Object.entries(data.colors ?? {})) {
      palette.set(name, { value, source: source(["colors", name]) });
    }

    // ui 里的组件键：嵌套对象 + 字面量点号键混排
    const walk = (node, path) => {
      if (node === null || typeof node !== "object" || Array.isArray(node)) {
        tokens.set(path.join("."), {
          value: node,
          source: source(path),
          platform: data.dark ? "dark" : "light",
        });
        return;
      }

      const keys = Object.keys(node);
      const osKey = keys.find((k) => k === "os.windows") ?? keys.find((k) => k === "os.default");
      const onlyOs = keys.every((k) => k.startsWith("os."));

      if (osKey && onlyOs) {
        tokens.set(path.join("."), {
          value: node[osKey],
          source: source([...path, osKey]),
          platform: data.dark ? "dark" : "light",
          os: osKey,
        });
        return;
      }

      for (const key of keys) {
        walk(node[key], [...path, key]);
      }
    };

    for (const [key, value] of Object.entries(data.ui ?? {})) {
      walk(value, [key]);
    }

    for (const [key, value] of Object.entries(data.icons ?? {})) {
      icons.set(key, { value, source: source(["icons", key]) });
    }
  }

  // 别名解析：值不是颜色 / 不是类名 / 不是图标路径时，去 palette 里找
  const resolveValue = (raw) => {
    const chainValues = [];
    let current = raw;
    for (let depth = 0; depth < 10; depth++) {
      if (typeof current !== "string") return { value: current, chain: chainValues };
      if (isColorish(current)) return { value: current.toLowerCase(), chain: chainValues };
      if (current === "transparent") {
        return { value: "#00000000", chain: [...chainValues, "transparent"], assumption: true };
      }
      // 调色板必须先于字面量判断：toolbar-bg-hovered / control-brand-border 都要继续往下走
      const paletteHit = palette.get(current);
      if (paletteHit) {
        chainValues.push(current);
        current = paletteHit.value;
        continue;
      }
      if (ICON_PATH.test(current)) return { value: current, chain: chainValues, kind: "icon" };
      if (CLASS_NAME.test(current)) return { value: current, chain: chainValues, kind: "class" };
      // "5,5,5,5" / "1,1,1,1,#B9BDC9" 这类 insets（可带颜色）字符串：原样保留
      if (/^-?\d+(\.\d+)?(\s*,\s*-?\d+(\.\d+)?)*(\s*,\s*#([0-9a-fA-F]{6}|[0-9a-fA-F]{8}))?$/.test(current)) {
        return { value: current, chain: chainValues, kind: "insets" };
      }
      // underline / fill / - 这类枚举字面量：原样保留
      if (current === "-" || /^[a-zA-Z][\w-]*$/.test(current)) {
        return { value: current, chain: chainValues, kind: "literal" };
      }
      if (/^-?\d+(\.\d+)?$/.test(current) || current === "true" || current === "false") {
        return { value: current, chain: chainValues, kind: "scalar" };
      }
      const hit = palette.get(current);
      if (!hit) return { value: null, chain: [...chainValues, current], unresolved: true };
      chainValues.push(current);
      current = hit.value;
    }
    return { value: null, chain: chainValues, unresolved: true };
  };

  const resolved = {};
  const unresolved = {};
  for (const [key, entry] of tokens) {
    const r = resolveValue(entry.value);
    if (r.unresolved) {
      unresolved[key] = { ...entry, chain: r.chain };
      continue;
    }
    resolved[key] = {
      value: r.value,
      chain: r.chain,
      source: entry.source,
      ...(entry.os ? { os: entry.os } : {}),
      ...(r.assumption ? { assumption: "transparent → #00000000" } : {}),
      ...(r.kind ? { kind: r.kind } : {}),
    };
  }

  return { palette, tokens, icons, resolved: sortObject(resolved), unresolved: sortObject(unresolved), notes };
}

function sortObject(obj) {
  return Object.fromEntries(Object.entries(obj).sort(([a], [b]) => a.localeCompare(b)));
}

// ---------------------------------------------------------------------------
// 与 keel 现有主题的差异
// ---------------------------------------------------------------------------

function keelDiff(slug, merged) {
  const keelPath = resolve("themes/idea-light.json");
  if (!existsSync(keelPath)) return `# keel 对照\n\n找不到 themes/idea-light.json\n`;
  const keel = JSON.parse(readFileSync(keelPath, "utf8"));
  const keelColors = keel.colors ?? {};
  const resolved = merged.resolved;

  // IDEA 的「出处」既包括解析后的令牌值，也包括调色板本身（gray-150 = #E9EAEE）
  const ideaValues = new Map(); // value -> [说明]
  const addValue = (value, label) => {
    if (typeof value !== "string" || !isColorish(value)) return;
    const normalized = value.toLowerCase();
    const list = ideaValues.get(normalized) ?? [];
    list.push(label);
    ideaValues.set(normalized, list);
  };
  for (const [key, entry] of Object.entries(resolved)) addValue(entry.value, `token:${key}`);
  for (const [name, entry] of merged.palette) addValue(entry.value, `palette:${name}`);

  const missing = [];
  for (const [name, value] of Object.entries(keelColors)) {
    const normalized = String(value).toLowerCase();
    if (!ideaValues.has(normalized)) missing.push({ name, value });
  }

  const focus = /^(ActionButton|Island|Islands|MainToolbar|TabbedPane|TitlePane|StatusBar|ToolWindow|EditorTabs|Panel|Borders|Window|MainWindow|Toolbar|Button|actions|Action|Tree|ComboBox)\./;
  const focusLines = Object.entries(resolved)
    .filter(([key]) => focus.test(key))
    .map(([key, e]) => `| \`${key}\` | \`${e.value}\` | ${e.chain.length > 1 ? e.chain.join(" → ") : "—"} | \`${e.source}\` |`);

  return [
    `# ${slug} vs keel`,
    "",
    `- IDEA 焦距键：${focusLines.length}`,
    `- keel 里在 IDEA 解析结果中找不到同色值的键：${missing.length}`,
    "",
    "## keel 中值没有 IDEA 出处的键（可能是自造色 / 近似色）",
    "",
    "| keel 键 | 值 |",
    "|---|---|",
    ...missing.map((m) => `| \`${m.name}\` | \`${m.value}\` |`),
    "",
    "## IDEA 焦点键",
    "",
    "| 键 | 值 | 别名链 | 来源 |",
    "|---|---|---|---|",
    ...focusLines,
    "",
  ].join("\n");
}

// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------

mkdirSync(OUT, { recursive: true });

for (const themeId of THEMES) {
  const chain = buildChain(themeId);
  const top = chain[chain.length - 1];
  const merged = mergeTheme(chain);
  const slug = themeId.toLowerCase().replace(/[^a-z0-9]+/g, "-");

  const payload = {
    theme: themeId,
    dark: Boolean(top.data.dark),
    chain: chain.map((t) => ({
      id: t.id,
      name: t.data.name,
      file: t.file.slice(SRC.length + 1).replace(/\\/g, "/"),
      parent: t.data.parentTheme ?? null,
    })),
    counts: {
      palette: merged.palette.size,
      tokens: merged.tokens.size,
      resolved: Object.keys(merged.resolved).length,
      unresolved: Object.keys(merged.unresolved).length,
    },
    tokens: merged.resolved,
  };

  writeFileSync(join(OUT, `${slug}.resolved.json`), JSON.stringify(payload, null, 2) + "\n");
  writeFileSync(
    join(OUT, `${slug}.unresolved.json`),
    JSON.stringify(merged.unresolved, null, 2) + "\n",
  );
  writeFileSync(join(OUT, `keel-diff-${slug}.md`), keelDiff(themeId, merged));

  console.log(
    `${themeId}: ${payload.counts.resolved} 个键已解析，${payload.counts.unresolved} 个未解析（${chain.length} 层主题）`,
  );
  for (const layer of payload.chain) console.log(`  - ${layer.id} → ${layer.file}`);
}
