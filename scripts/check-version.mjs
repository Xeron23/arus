import fs from "node:fs";
import assert from "node:assert/strict";
const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
const config = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const lock = JSON.parse(fs.readFileSync("package-lock.json", "utf8"));
assert.match(pkg.version, /^\d+\.\d+\.\d+$/);
assert.equal(config.version, pkg.version, "Tauri and package.json versions differ");
assert.equal(lock.version, pkg.version);
assert.equal(lock.packages[""].version, pkg.version);
for (const path of ["src-tauri/Cargo.toml", "src-tauri/core/Cargo.toml"]) {
  const version = fs.readFileSync(path, "utf8").match(/^version = "([^"]+)"/m)?.[1];
  assert.equal(version, pkg.version, `${path} version differs`);
}
assert(Number.isSafeInteger(config.bundle.android.versionCode));
assert(config.bundle.android.versionCode > 1_000_000 && config.bundle.android.versionCode <= 2_100_000_000);
if (process.env.RELEASE_REF?.startsWith("refs/tags/")) {
  assert.equal(process.env.RELEASE_REF, `refs/tags/v${pkg.version}`, "Release tag must match application version");
}
console.log(`Version ${pkg.version}; Android versionCode ${config.bundle.android.versionCode}`);
