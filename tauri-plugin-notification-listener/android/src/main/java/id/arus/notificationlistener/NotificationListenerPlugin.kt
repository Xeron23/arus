package id.arus.notificationlistener

import android.app.Activity
import android.app.NotificationManager
import android.content.ComponentName
import android.content.Intent
import android.os.Build
import android.provider.Settings
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

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

    // Separate executor keeps downloads from blocking notification capture.
    private val updater = ArusUpdater(activity)
    @Command fun checkUpdate(invoke: Invoke) = updater.check(invoke)
    @Command fun downloadUpdate(invoke: Invoke) = updater.download(invoke)
    @Command fun installUpdate(invoke: Invoke) = updater.install(invoke)
    @Command fun updateInstallPermission(invoke: Invoke) = updater.permission(invoke)
    @Command fun updateInstallSettings(invoke: Invoke) = updater.settings(invoke)
}
