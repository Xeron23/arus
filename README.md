# Arus — Tauri v2 Mobile / Android

Project implementasi lokal untuk fitur utama mobile yang diminta: Vanilla TypeScript + Tailwind, Rust Tauri v2, SQLite dalam sandbox aplikasi, dan TCP/TLS IMAP langsung ke provider. Tidak ada backend HTTP/cloud milik pengembang, API key cloud, analytics, CDN runtime, atau server sinkronisasi perantara.

## Mulai dari browser

Prasyarat: Node.js 22+ dan npm.

```sh
npm ci
npm run dev
```

Buka URL localhost yang dicetak Vite (port 1420). `isTauri()` memilih adapter otomatis. Browser memakai IndexedDB dan data contoh; native memakai IPC Rust dan SQLite kosong dengan empat dompet bersaldo awal 0. Error native tidak pernah dialihkan diam-diam ke mock. Data browser bukan data aplikasi native dan tidak disalin ke native.

## Android: prasyarat dan build

Pasang Rust stable yang masih didukung (gunakan versi stable terbaru; kode inti membutuhkan setidaknya 1.88), Android Studio, SDK Platform, SDK Build-Tools, Platform-Tools, Command-line Tools, serta NDK side-by-side terbaru. Gunakan JBR/JDK bawaan Android Studio. Isi `JAVA_HOME`, `ANDROID_HOME`, dan `NDK_HOME` sesuai lokasi di mesin sendiri. NDK r28+ disarankan untuk dukungan halaman memori 16 KiB.

Contoh PowerShell Windows, sesuaikan nama direktori NDK dengan versi yang benar-benar dipasang:

```powershell
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$arusNdkVersion = (Get-ChildItem "$env:ANDROID_HOME\ndk" -Directory | Sort-Object LastWriteTime -Descending | Select-Object -First 1).Name
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\$arusNdkVersion"
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
npm ci
npm run android:init
npm run android:dev
```

Sambungkan HP dengan USB debugging aktif, atau jalankan emulator Android Studio. Jika dev server memakai IP LAN, PC dan HP perlu saling menjangkau port Vite; Tauri akan mencetak alamat pengembangan yang digunakan. Ini hanya tooling pengembangan. APK produksi memuat `dist/` dari dalam aplikasi.

Membuat APK debug untuk pengujian lokal:

```sh
npm run android:build
```

APK akan berada di bawah `src-tauri/gen/android/app/build/outputs/apk/`, sesuai ABI/build variant yang dipilih CLI. Script membangun `--apk --debug` agar hasil pengujian memakai debug signing. Untuk distribusi release:

```sh
npm run android:release
```

Release memerlukan keystore/signing milik pemilik aplikasi; konfigurasi dan kunci signing tidak disertakan. Ikuti panduan resmi Tauri Android signing.

`android:init` menghasilkan project Gradle/Kotlin standar Tauri, kemudian `android-prepare.mjs` memasang permission INTERNET, menonaktifkan backup Android otomatis, dan melarang cleartext pada release. Debug memiliki manifest/network-security-config terpisah untuk server Vite. Folder `gen/` sengaja dihasilkan oleh versi CLI yang terkunci di package-lock, bukan berisi output Android Studio buatan tangan.

## Menguji koneksi Gmail nyata

1. Jalankan binary native di Android atau desktop Tauri, bukan URL web biasa.
2. Pengaturan → Hubungkan email. Buka wizard, aktifkan Verifikasi 2 Langkah Google, dan buat App Password. Opsi ini tidak tersedia pada seluruh jenis akun/kebijakan organisasi.
3. Masukkan alamat Gmail dan App Password 16 huruf; spasi pemisah dihapus oleh Rust. Jangan gunakan sandi utama akun.
4. Pilih dompet default. Auto-post dimatikan secara default untuk akun baru.
5. Tekan **Test Connection**. Rust membuka TCP, melakukan TLS dengan validasi sertifikat dan SNI, membaca greeting, melakukan LOGIN, LIST, dan EXAMINE read-only.
6. Tekan **Simpan akun**. Kredensial diuji kembali sebelum metadata akun disimpan; sandinya hanya disimpan di memori Rust menggunakan `Zeroizing<String>`.
7. Atur mapping pengirim bank → dompet. Tekan Sync. Setelah aplikasi benar-benar ditutup, buka sesi lagi dengan App Password; password tidak dipersistenkan ke SQLite, IndexedDB, localStorage, atau file lain.

Provider Custom selalu menggunakan implicit TLS; tidak ada opsi menonaktifkan validasi sertifikat atau mengirim sandi melalui koneksi plaintext. Gmail, Yahoo, Outlook memiliki host bawaan. Provider yang menolak LOGIN/App Password dan mewajibkan OAuth2 akan mengembalikan error autentikasi. OAuth2 belum diimplementasikan.

Diagnostik Rust tanpa WebView, dengan kredensial melalui environment lokal (jangan memasukkannya ke source/repository):

```sh
cargo run --manifest-path src-tauri/core/Cargo.toml --example imap_check
```

Contoh membaca input di PowerShell tanpa mencetak sandi:

```powershell
$env:ARUS_IMAP_EMAIL = Read-Host "Email Gmail"
$arusSecret = Read-Host "App Password" -AsSecureString
$env:ARUS_IMAP_PASSWORD = [System.Net.NetworkCredential]::new("", $arusSecret).Password
cargo run --manifest-path src-tauri/core/Cargo.toml --example imap_check
Remove-Item Env:\ARUS_IMAP_PASSWORD
Remove-Item Env:\ARUS_IMAP_EMAIL
```

## Struktur kode

```text
arus-tauri-mobile/
  src/
    main.ts                  Event, formulir, dialog, progress, navigasi
    models.ts                Kontrak tipe IPC / adapter
    styles.css               Tailwind + desain Arus + dark theme
    native/client.ts         Native invoke + Channel; pemilihan adapter
    native/mock.ts           IndexedDB browser, simulasi dan ledger demo
    ui/icons.ts              Ikon SVG lokal
    ui/templates.ts          Dashboard, kartu swipe, transaksi, audit, setup
  src-tauri/