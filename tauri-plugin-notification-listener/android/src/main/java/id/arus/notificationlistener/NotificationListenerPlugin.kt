package id.arus.notificationlistener

import android.app.Activity
import android.app.NotificationManager
import android.content.ComponentName
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

@InvokeArg
class EnabledArgs {
    var enabled: Boolean = false
}

@InvokeArg
class AckArgs {
    var ids: Array<String> = emptyArray()
}

@TauriPlugin
class NotificationListenerPlugin(
    private val activity: Activity
) : Plugin(activity) {
    private val component
        get() = ComponentName(activity, ArusNotificationService::class.java)

    private val updatesDir: File
        get() = File(activity.cacheDir, "updates").apply { mkdirs() }

    private val apkFile get() = File(updatesDir, "update.apk")

    private fun accessGranted(): Boolean {
        if (Build.VERSION.SDK_INT >= 27) {
            val nm = activity.getSystemService(NotificationManager::class.java)
            if (nm != null) {
                return nm.isNotificationListenerAccessGranted(component)
            }
        }

        return Settings.Secure.getString(
            activity.contentResolver, "enabled_notification_listeners"
        ).orEmpty().split(":")
            .mapNotNull { ComponentName.unflattenFromString(it) }
            .any { it == component }
    }

    private fun io(invoke: Invoke, body: () -> JSObject) {
        NativeQueue.io.execute {
            try {
                invoke.resolve(body())
            } catch (e: Exception) {
                invoke.reject(e.message ?: "Operasi native gagal")
            }
        }
    }

    @Command
    fun status(invoke: Invoke) {
        val granted = accessGranted()
        io(invoke) {
            JSObject().apply {
                put("granted", granted)
                put("enabled", NativeQueue.enabled(activity))
                put("connected", NativeQueue.connected)
                put("lastError", NativeQueue.lastError ?: "")
            }
        }
    }

    @Command
    fun openSettings(invoke: Invoke) {
        try {
            activity.startActivity(
                Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)
            )
            invoke.resolve(JSObject())
        } catch (_: Exception) {
            invoke.reject("Pengaturan akses notifikasi tidak tersedia")
        }
    }

    @Command
    fun setEnabled(invoke: Invoke) {
        val args = invoke.parseArgs(EnabledArgs::class.java)
        if (args.enabled && !accessGranted()) {
            invoke.reject("Aktifkan akses notifikasi di Settings terlebih dahulu")
            return
        }
        io(invoke) {
            NativeQueue.setEnabled(activity, args.enabled)
            if (args.enabled) {
                try {
                    android.service.notification.NotificationListenerService
                        .requestRebind(component)
                } catch (_: Exception) {}
            }
            JSObject()
        }
    }

    @Command
    fun peek(invoke: Invoke) = io(invoke) {
        JSObject().apply { put("items", NativeQueue.peek(activity)) }
    }

    @Command
    fun ack(invoke: Invoke) {
        val args = invoke.parseArgs(AckArgs::class.java)
        io(invoke) {
            NativeQueue.ack(activity, args.ids)
            JSObject()
        }
    }

    // --- In-App Updater Commands ---

    @Command
    fun checkUpdate(invoke: Invoke) = io(invoke) {
        val url = URL("https://github.com/Xeron23/arus/releases/latest/download/latest.json")
        val conn = (url.openConnection() as HttpURLConnection).apply {
            instanceFollowRedirects = true
            connectTimeout = 10000
            readTimeout = 15000
            setRequestProperty("User-Agent", "Arus-App")
        }

        val jsonStr = conn.inputStream.bufferedReader().use { it.readText() }
        val manifest = JSONObject(jsonStr)

        @Suppress("DEPRECATION")
        val pInfo = activity.packageManager.getPackageInfo(activity.packageName, 0)
        val currentVersionCode = if (Build.VERSION.SDK_INT >= 28) pInfo.longVersionCode else pInfo.versionCode.toLong()
        val latestVersionCode = manifest.optLong("versionCode", 0L)
        val latestVersion = manifest.optString("version", "")
        val notes = manifest.optString("notes", "")
        val apkObj = manifest.optJSONObject("apk")
        val apkUrl = apkObj?.optString("url", "") ?: ""
        val apkSize = apkObj?.optLong("size", 0L) ?: 0L

        val available = latestVersionCode > currentVersionCode

        JSObject().apply {
            put("available", available)
            put("currentVersion", pInfo.versionName ?: "")
            put("currentVersionCode", currentVersionCode)
            put("latestVersion", latestVersion)
            put("latestVersionCode", latestVersionCode)
            put("notes", notes)
            put("apkUrl", apkUrl)
            put("apkSize", apkSize)
        }
    }

    @Command
    fun downloadUpdate(invoke: Invoke) = io(invoke) {
        val args = invoke.parseArgs(JSObject::class.java)
        val urlStr = args.optString("url", "")
        if (urlStr.isEmpty()) throw IllegalArgumentException("URL update kosong")

        var currentUrl = URL(urlStr)
        var redirects = 0
        var conn: HttpURLConnection? = null

        while (redirects < 6) {
            conn = (currentUrl.openConnection() as HttpURLConnection).apply {
                instanceFollowRedirects = false
                connectTimeout = 15000
                readTimeout = 30000
                setRequestProperty("User-Agent", "Arus-App")
            }
            val code = conn.responseCode
            if (code in 301..308) {
                val loc = conn.getHeaderField("Location") ?: break
                currentUrl = URL(currentUrl, loc)
                conn.disconnect()
                redirects++
            } else {
                break
            }
        }

        val finalConn = conn ?: throw IllegalStateException("Koneksi gagal")
        val tempFile = File(updatesDir, "update.part")
        finalConn.inputStream.use { input ->
            tempFile.outputStream().use { output ->
                input.copyTo(output)
            }
        }
        finalConn.disconnect()

        if (apkFile.exists()) apkFile.delete()
        tempFile.renameTo(apkFile)

        JSObject().apply {
            put("ready", true)
            put("size", apkFile.length())
        }
    }

    @Command
    fun installUpdate(invoke: Invoke) {
        activity.runOnUiThread {
            try {
                if (Build.VERSION.SDK_INT >= 26 && !activity.packageManager.canRequestPackageInstalls()) {
                    val intent = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES).apply {
                        data = Uri.parse("package:${activity.packageName}")
                    }
                    activity.startActivity(intent)
                    invoke.reject("Izin instalasi aplikasi dibutuhkan. Silakan aktifkan 'Izinkan dari sumber ini' lalu coba pasang kembali.")
                    return@runOnUiThread
                }

                if (!apkFile.exists()) {
                    invoke.reject("File APK belum diunduh.")
                    return@runOnUiThread
                }

                val uri = FileProvider.getUriForFile(
                    activity,
                    "${activity.packageName}.arus.updates",
                    apkFile
                )

                val intent = Intent(Intent.ACTION_VIEW).apply {
                    setDataAndType(uri, "application/vnd.android.package-archive")
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                activity.startActivity(intent)
                invoke.resolve(JSObject().apply { put("started", true) })
            } catch (e: Exception) {
                invoke.reject(e.message ?: "Gagal membuka installer")
            }
        }
    }
}
