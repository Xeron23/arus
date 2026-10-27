package id.arus.notificationlistener

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.SystemClock
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/** Network work must not block NativeQueue.io, which receives transaction notifications. */
internal class ArusUpdater(private val activity: Activity) {
    companion object {
        private const val REPO = "Xeron23/arus"
        private const val MAX_APK = 200L * 1024 * 1024
        private const val ENDPOINT = "https://github.com/$REPO/releases/latest/download/latest.json"
    }
    private val executor = Executors.newSingleThreadExecutor()
    private val installing = AtomicBoolean(false)
    private var candidate: JSONObject? = null
    private val pm get() = activity.packageManager
    private val directory get() = File(activity.cacheDir, "updates").apply {
        check(isDirectory || mkdirs()) { "Penyimpanan pembaruan tidak tersedia." }
    }
    private val apk get() = File(directory, "update.apk")
    private val metadata get() = File(directory, "update.json")

    private fun work(invoke: Invoke, body: () -> JSObject) {
        executor.execute {
            try {
                check(!installing.get()) { "Pemasangan sedang dibuka. Coba lagi sebentar." }
                invoke.resolve(body())
            } catch (e: Exception) {
                val message = if (e is IllegalArgumentException || e is IllegalStateException)
                    e.message ?: "Pembaruan belum berhasil. Coba lagi."
                else "Koneksi pembaruan terputus atau file tidak tersedia. Coba lagi."
                invoke.reject(message)
            }
        }
    }

    private fun flags() = if (Build.VERSION.SDK_INT >= 28)
        PackageManager.GET_SIGNING_CERTIFICATES else PackageManager.GET_SIGNATURES

    @Suppress("DEPRECATION")
    private fun installed() = pm.getPackageInfo(activity.packageName, flags())

    @Suppress("DEPRECATION")
    private fun version(info: PackageInfo) =
        if (Build.VERSION.SDK_INT >= 28) info.longVersionCode else info.versionCode.toLong()

    private fun hex(bytes: ByteArray) = bytes.joinToString("") { "%02x".format(it.toInt() and 255) }

    @Suppress("DEPRECATION")
    private fun certificates(info: PackageInfo): Set<String> {
        val signatures = if (Build.VERSION.SDK_INT >= 28)
            info.signingInfo?.apkContentsSigners else info.signatures
        return signatures?.map {
            hex(MessageDigest.getInstance("SHA-256").digest(it.toByteArray()))
        }?.toSet().orEmpty()
    }

    private fun hash(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().buffered().use { input ->
            val buffer = ByteArray(65536)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                digest.update(buffer, 0, count)
            }
        }
        return hex(digest.digest())
    }

    private fun connection(address: String): HttpURLConnection {
        var url = URL(address)
        repeat(6) {
            require(url.protocol == "https" && url.userInfo == null &&
                (url.port == -1 || url.port == 443) && url.host.lowercase() in setOf(
                    "github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com"
                )) { "Alamat pembaruan tidak diizinkan." }
            val conn = (url.openConnection() as HttpURLConnection).apply {
                instanceFollowRedirects = false
                connectTimeout = 15000
                readTimeout = 30000
                useCaches = false
                setRequestProperty("User-Agent", "Arus-Android-Updater")
                setRequestProperty("Accept-Encoding", "identity")
                setRequestProperty("Cache-Control", "no-cache")
            }
            try {
                val status = conn.responseCode
                if (status in listOf(301, 302, 303, 307, 308)) {
                    val location = conn.getHeaderField("Location") ?: error("Alamat unduhan tidak tersedia.")
                    url = URL(url, location)
                    conn.disconnect()
                } else {
                    check(status == 200) { "Informasi pembaruan belum tersedia (HTTP $status)." }
                    return conn
                }
            } catch (e: Exception) { conn.disconnect(); throw e }
        }
        error("Terlalu banyak pengalihan alamat unduhan.")
    }

    private fun validateManifest(m: JSONObject) {
        require(m.getInt("schema") == 1 && m.getString("applicationId") == activity.packageName) {
            "Informasi pembaruan tidak cocok dengan aplikasi ini."
        }
        require(m.getLong("versionCode") in 1..2100000000L &&
            m.getString("version").length in 1..100 && m.optString("notes").length <= 8000) {
            "Informasi versi pembaruan tidak valid."
        }
        val a = m.getJSONObject("apk")
        require(a.getString("abi") == "arm64-v8a" && "arm64-v8a" in Build.SUPPORTED_ABIS) {
            "Pembaruan ini belum mendukung perangkat Anda."
        }
        require(a.getInt("minSdk") in 24..Build.VERSION.SDK_INT) {
            "Pembaruan memerlukan Android yang lebih baru."
        }
        require(a.getLong("size") in 1..MAX_APK && a.getString("sha256").matches(Regex("[a-fA-F0-9]{64}"))) {
            "Informasi file pembaruan tidak lengkap."
        }
        val url = URL(a.getString("url"))
        require(url.protocol == "https" && url.host == "github.com" &&
            url.path.startsWith("/$REPO/releases/download/")) { "Sumber pembaruan tidak sesuai." }
    }

    fun check(invoke: Invoke) = work(invoke) {
        candidate = null
        val conn = connection(ENDPOINT)
        val m = try {
            val bytes = conn.inputStream.use { input ->
                val output = java.io.ByteArrayOutputStream()
                val buffer = ByteArray(4096)
                while (true) {
                    val n = input.read(buffer)
                    if (n < 0) break
                    check(output.size() + n <= 65536) { "Informasi pembaruan terlalu besar." }
                    output.write(buffer, 0, n)
                }
                output.toByteArray()
            }
            JSONObject(bytes.toString(Charsets.UTF_8))
        } finally { conn.disconnect() }
        validateManifest(m)
        val current = installed()
        val available = m.getLong("versionCode") > version(current)
        if (available) candidate = m
        JSObject().apply {
            put("available", available)
            put("currentVersion", current.versionName ?: "")
            put("currentVersionCode", version(current))
            put("latestVersion", m.getString("version"))
            put("latestVersionCode", m.getLong("versionCode"))
            put("notes", m.optString("notes"))
            put("apkUrl", m.getJSONObject("apk").getString("url"))
            put("apkSize", m.getJSONObject("apk").getLong("size"))
        }
    }

    @Suppress("DEPRECATION")
    private fun verify(file: File, m: JSONObject) {
        validateManifest(m)
        val a = m.getJSONObject("apk")
        require(file.isFile && file.length() == a.getLong("size") &&
            hash(file).equals(a.getString("sha256"), true)) { "Unduhan tidak lengkap atau rusak. Unduh ulang." }
        val incoming = pm.getPackageArchiveInfo(file.absolutePath, flags())
            ?: error("File pembaruan bukan APK yang valid.")
        val current = installed()
        require(incoming.packageName == activity.packageName && version(incoming) == m.getLong("versionCode") &&
            incoming.versionName == m.getString("version") && version(incoming) > version(current)) {
            "Versi APK tidak cocok atau bukan versi yang lebih baru."
        }
        require((incoming.applicationInfo?.minSdkVersion ?: Int.MAX_VALUE) <= Build.VERSION.SDK_INT) {
            "Versi Android perangkat belum mendukung pembaruan ini."
        }
        val old = certificates(current)
        require(old.isNotEmpty() && old == certificates(incoming)) {
            "Tanda tangan pembaruan berbeda. Hubungi pengembang; jangan hapus aplikasi karena data lokal dapat hilang."
        }
    }

    fun download(invoke: Invoke) = work(invoke) {
        val m = candidate ?: error("Periksa pembaruan terlebih dahulu.")
        val a = m.getJSONObject("apk")
        val part = File(directory, "update.part")
        if (metadata.exists()) check(metadata.delete()) { "Tidak dapat menyiapkan penyimpanan pembaruan." }
        try {
            val conn = connection(a.getString("url"))
            try {
                var count = 0L
                val start = SystemClock.elapsedRealtime()
                conn.inputStream.buffered().use { input -> part.outputStream().buffered().use { output ->
                    val buffer = ByteArray(65536)
                    while (true) {
                        check(SystemClock.elapsedRealtime() - start < 600000) { "Unduhan terlalu lama. Coba lagi." }
                        val n = input.read(buffer)
                        if (n < 0) break
                        count += n
                        check(count <= a.getLong("size") && count <= MAX_APK) { "Ukuran unduhan tidak sesuai." }
                        output.write(buffer, 0, n)
                    }
                } }
            } finally { conn.disconnect() }
            verify(part, m)
            if (apk.exists()) check(apk.delete()) { "Tidak dapat mengganti file pembaruan." }
            check(part.renameTo(apk)) { "Tidak dapat menyimpan file pembaruan." }
            metadata.writeText(m.toString())
            JSObject().apply { put("ready", true) }
        } finally { part.delete() }
    }

    private fun canInstall() = Build.VERSION.SDK_INT < 26 || pm.canRequestPackageInstalls()
    fun permission(invoke: Invoke) = invoke.resolve(JSObject().apply { put("granted", canInstall()) })
    fun settings(invoke: Invoke) {
        try {
            activity.startActivity(if (Build.VERSION.SDK_INT >= 26)
                Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${activity.packageName}"))
                else Intent(Settings.ACTION_SECURITY_SETTINGS))
            invoke.resolve(JSObject())
        } catch (_: Exception) { invoke.reject("Pengaturan pemasangan tidak tersedia.") }
    }

    fun install(invoke: Invoke) {
        executor.execute {
            try {
                check(!installing.get()) { "Pemasangan sedang disiapkan." }
                check(metadata.isFile && metadata.length() <= 65536) { "Unduh ulang pembaruan terlebih dahulu." }
                verify(apk, JSONObject(metadata.readText()))
                check(canInstall()) { "Izinkan pemasangan dari Arus terlebih dahulu." }
                installing.set(true)
                activity.runOnUiThread {
                    try {
                        val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.arus.updates", apk)
                        activity.startActivity(Intent(Intent.ACTION_VIEW).apply {
                            setDataAndType(uri, "application/vnd.android.package-archive")
                            clipData = ClipData.newRawUri("Arus update", uri)
                            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                        })
                        invoke.resolve(JSObject().apply { put("started", true) })
                    } catch (_: Exception) {
                        invoke.reject("Pemasang Android tidak dapat dibuka. Periksa izin pemasangan.")
                    } finally { installing.set(false) }
                }
            } catch (e: Exception) {
                installing.set(false)
                invoke.reject(e.message ?: "Pembaruan belum siap. Unduh ulang.")
            }
        }
    }
}
