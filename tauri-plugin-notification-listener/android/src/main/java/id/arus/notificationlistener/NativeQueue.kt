package id.arus.notificationlistener

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.Executors

internal object NativeQueue {
    val io = Executors.newSingleThreadExecutor()

    @Volatile var connected = false
    @Volatile var lastError: String? = null

    private var helper: Helper? = null

    private class Helper(c: Context) :
        SQLiteOpenHelper(c.applicationContext, "notification_queue.db", null, 1) {
        override fun onCreate(db: SQLiteDatabase) {
            db.execSQL("""
                CREATE TABLE IF NOT EXISTS queue (
                  id TEXT PRIMARY KEY,
                  payload TEXT NOT NULL
                )
            """.trimIndent())
        }

        override fun onUpgrade(db: SQLiteDatabase, old: Int, new: Int) = Unit
    }

    private fun db(c: Context): SQLiteDatabase {
        val h = helper ?: Helper(c).also { helper = it }
        return h.writableDatabase
    }

    private fun prefs(c: Context) =
        c.getSharedPreferences("notification_listener", Context.MODE_PRIVATE)

    fun enabled(c: Context) = prefs(c).getBoolean("enabled", false)

    fun setEnabled(c: Context, enabled: Boolean) {
        check(prefs(c).edit().putBoolean("enabled", enabled).commit()) {
            "Preferensi pengumpulan notifikasi gagal disimpan"
        }
    }

    fun put(c: Context, payload: JSONObject) {
        val values = ContentValues().apply {
            put("id", payload.getString("id"))
            put("payload", payload.toString())
        }
        val database = db(c)
        database.insertWithOnConflict(
            "queue", null, values, SQLiteDatabase.CONFLICT_IGNORE
        )
        // insertWithOnConflict may return -1 for both duplicates and failures.
        // A matching durable row, not that return value, proves success.
        database.rawQuery("SELECT payload FROM queue WHERE id = ?", arrayOf(payload.getString("id"))).use { cursor ->
            check(cursor.moveToFirst() && cursor.getString(0) == payload.toString()) {
                "QUEUE_WRITE_FAILED_OR_ID_CONFLICT"
            }
        }
    }

    fun peek(c: Context): JSONArray {
        val result = JSONArray()
        db(c).rawQuery(
            "SELECT payload FROM queue ORDER BY rowid LIMIT 100", null
        ).use { cursor ->
            while (cursor.moveToNext()) {
                result.put(JSONObject(cursor.getString(0)))
            }
        }
        return result
    }

    fun ack(c: Context, ids: Array<String>) {
        if (ids.isEmpty()) return
        val database = db(c)
        database.beginTransaction()
        try {
            ids.forEach { database.delete("queue", "id = ?", arrayOf(it)) }
            database.setTransactionSuccessful()
        } finally {
            database.endTransaction()
        }
    }
}
