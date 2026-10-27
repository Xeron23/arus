import fs from "node:fs";
const main = "src-tauri/gen/android/app/src/main/AndroidManifest.xml";
if (!fs.existsSync(main)) {
  throw Error(
    "Project Android belum dibuat. Jalankan npm run android:init setelah Android SDK/NDK siap.",
  );
}
let xml = fs.readFileSync(main, "utf8");
if (!xml.includes("android.permission.INTERNET"))
  xml = xml.replace(
    /<application\b/,
    '<uses-permission android:name="android.permission.INTERNET"/>\n    <application',
  );
for (const [key, val] of [
  ["allowBackup", "false"],
  ["fullBackupContent", "false"],
  ["usesCleartextTraffic", "false"],
  ["networkSecurityConfig", "@xml/arus_network_security"],
]) {
  const re = new RegExp(`android:${key}="[^"]*"`);
  xml = re.test(xml)
    ? xml.replace(re, `android:${key}="${val}"`)
    : xml.replace(/<application\b/, `<application android:${key}="${val}"`);
}
fs.writeFileSync(main, xml);
const debug = "src-tauri/gen/android/app/src/debug/AndroidManifest.xml";
fs.mkdirSync("src-tauri/gen/android/app/src/debug", { recursive: true });
fs.mkdirSync("src-tauri/gen/android/app/src/main/res/xml", { recursive: true });
fs.writeFileSync(
  "src-tauri/gen/android/app/src/main/res/xml/arus_network_security.xml",
  '<network-security-config><base-config cleartextTrafficPermitted="false"><trust-anchors><certificates src="system"/></trust-anchors></base-config></network-security-config>',
);
fs.mkdirSync("src-tauri/gen/android/app/src/debug/res/xml", {
  recursive: true,
});
fs.writeFileSync(
  "src-tauri/gen/android/app/src/debug/res/xml/arus_network_security_debug.xml",
  '<network-security-config><base-config cleartextTrafficPermitted="true"><trust-anchors><certificates src="system"/></trust-anchors></base-config></network-security-config>',
);
fs.writeFileSync(
  debug,
  '<manifest xmlns:android="http://schemas.android.com/apk/res/android" xmlns:tools="http://schemas.android.com/tools"><uses-permission android:name="android.permission.INTERNET"/><application android:usesCleartextTraffic="true" android:networkSecurityConfig="@xml/arus_network_security_debug" tools:replace="android:usesCleartextTraffic,android:networkSecurityConfig"/></manifest>',
);
console.log(
  "Android ready: INTERNET; automatic Android backup off; cleartext allowed only in debug.",
);


