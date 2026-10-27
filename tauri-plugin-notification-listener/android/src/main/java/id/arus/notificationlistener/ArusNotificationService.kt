package id.arus.notificationlistener

import android.app.Notification
import android.content.ComponentName
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import org.json.JSONArray
import org.json.JSONObject
import java.security.MessageDigest

class ArusNotificationService : NotificationListenerService() {
    companion object {
        val allowedPackages = setOf(
            "com.gojek.gopay",
            "com.gojek.app",
            "ovo.id",
            "id.dana",
            "id.bmri.livin",
            "com.bca",
            "com.bca.mybca.omni.android",
            "com.shopee.id"
        )

        private val sensitive =
            Regex("""(?i)\b(otp|pin|password|kode verifikasi|kode autentikasi)\b""")
    }

    override fun onListenerConnected() {
        NativeQueue.connected = true
    }

    override fun onListenerDisconnected() {
        NativeQueue.connected = false
        NativeQueue.io.execute {
            if (NativeQueue.enabled(applicationContext)) {
                requestRebind(
                    ComponentName(
                        applicationContext,
                        ArusNotificationService::class.java
                    )
                )
            }
        }
    }

    override fun onDestroy() {
        NativeQueue.connected = false
        super.onDestroy()
    }

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        if (sbn.packageName !in allowedPackages) return

        val notification = sbn.notification ?: return
        if ((notification.flags and Notification.FLAG_GROUP_SUMMARY) != 0) return

        val extras = notification.extras ?: return
        val title = extras.getCharSequence(Notification.EXTRA_TITLE)?.toString().orEmpty()

        val text = extras.getCharSequence(Notification.EXTRA_BIG_TEXT)
            ?.toString()?.takeIf { it.isNotBlank() }
            ?: extras.getCharSequence(Notification.EXTRA_TEXT)?.toString()
            ?: extras.getCharSequenceArray(Notification.EXTRA_TEXT_LINES)
                ?.joinToString("\n")
            ?: ""

        if (title.length > 1024 || text.length > 8192) return
        if (text.isBlank() || sensitive.containsMatchIn("$title\n$text")) return

        val canonical = JSONArray(listOf(sbn.key, sbn.postTime, sbn.packageName, title, text)).toString()
        val id = MessageDigest.getInstance("SHA-256")
            .digest(canonical.toByteArray(Charsets.UTF_8))
            .joinToString("") { "%02x".format(it.toInt() and 255) }

        val payload = JSONObject().apply {
            put("id", id)
            put("packageName", sbn.packageName)
            put("title", title)
            put("text", text)
            put("timestamp", sbn.postTime)
        }

        NativeQueue.io.execute {
            try {
                if (NativeQueue.enabled(applicationContext)) {
                    NativeQueue.put(applicationContext, payload)
                    NativeQueue.lastError = null
                }
            } catch (_: Exception) {
                NativeQueue.lastError = "QUEUE_WRITE_FAILED"
            }
        }
    }
}
