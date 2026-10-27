"""Generate update metadata from the final signed APK."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

apk, aapt = Path(sys.argv[1]), sys.argv[2]
config = json.loads(Path("src-tauri/tauri.conf.json").read_text())
badging = subprocess.check_output([aapt, "dump", "badging", str(apk)], text=True)
package = re.search(r"^package: name='([^']+)' versionCode='(\d+)' versionName='([^']+)'", badging, re.M)
assert package, "Cannot read APK package metadata"
application_id, version_code, version = package.groups()
assert application_id == config["identifier"]
assert version == config["version"]
assert int(version_code) == config["bundle"]["android"]["versionCode"]
minimum = int(re.search(r"^sdkVersion:'(\d+)'", badging, re.M).group(1))
assert minimum == config["bundle"]["android"]["minSdkVersion"]
abis = re.search(r"^native-code: (.+)$", badging, re.M)
assert abis and abis.group(1).strip() == "'arm64-v8a'", "Expected standalone ARM64 APK"
with apk.open("rb") as stream:
    digest = hashlib.file_digest(stream, "sha256").hexdigest()
manifest = {
    "schema": 1, "applicationId": application_id,
    "version": version, "versionCode": int(version_code),
    "notes": "Pencatatan email otomatis, pilihan periode, tampilan lebih nyaman, dan perbaikan pembaruan aplikasi.",
    "apk": {
        "abi": "arm64-v8a", "minSdk": minimum,
        "url": f"https://github.com/Xeron23/arus/releases/download/v{version}/arus-release.apk",
        "size": apk.stat().st_size, "sha256": digest,
    },
}
apk.with_name("latest.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Manifest generated from signed APK: {version} ({version_code})")
