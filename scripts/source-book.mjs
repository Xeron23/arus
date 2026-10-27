import fs from "node:fs";
import path from "node:path";
const ignored = new Set([
  "node_modules",
  "target",
  "dist",
  "gen",
  ".git",
  ".openai",
]);
const extLang = {
  ".ts": "typescript",
  ".rs": "rust",
  ".json": "json",
  ".toml": "toml",
  ".sql": "sql",
  ".css": "css",
  ".html": "html",
  ".mjs": "javascript",
  ".cjs": "javascript",
  ".md": "markdown",
};
function walk(dir) {
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((e) => {
      if (ignored.has(e.name)) return [];
      const file = path.join(dir, e.name);
      if (e.isDirectory()) return walk(file);
      if (
        e.name === "Arus_Tauri_Source_Code.md" ||
        e.name.endsWith(".lock") ||
        e.name === "package-lock.json" ||
        e.name === "MANIFEST.sha256" ||
        e.name.endsWith(".png")
      )
        return [];
      return [file];
    });
}
const files = walk(".").filter(
  (f) => extLang[path.extname(f)] || f.endsWith(".gitignore"),
);
let text =
  "# Arus — source code lengkap\n\nSemua file kode dan konfigurasi di bawah dicetak lengkap tanpa pemotongan. Dependency lockfiles dan ikon PNG tersedia dalam ZIP project. Folder Android dihasilkan dengan `npm run android:init`.\n\n";
for (const file of files) {
  const value = fs.readFileSync(file, "utf8");
  text +=
    "## " +
    file +
    "\n\n" +
    "`".repeat(4) +
    (extLang[path.extname(file)] || "text") +
    "\n" +
    value +
    "\n" +
    "`".repeat(4) +
    "\n\n";
}
fs.writeFileSync("Arus_Tauri_Source_Code.md", text);
console.log(`${files.length} complete files, ${Buffer.byteLength(text)} bytes`);


