package com.salad.yamp.storage

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.Settings
import androidx.activity.result.ActivityResult
import app.tauri.PermissionState
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

// The folders are read by path, which is all that a scan can do with them. Since Android 11 a path
// of shared storage is only listed for the files that the media permissions cover, and those leave
// out a file that no media scan has indexed yet, like one that was just copied in. Only access to
// all files shows what is really in the folder. Before Android 11 the old permission does that.
@TauriPlugin(
    permissions = [
        Permission(strings = [Manifest.permission.READ_EXTERNAL_STORAGE], alias = "legacy"),
    ]
)
class StoragePlugin(private val host: Activity) : Plugin(host) {
    private fun granted() =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) Environment.isExternalStorageManager()
        else getPermissionState("legacy") == PermissionState.GRANTED

    private fun answer(invoke: Invoke) {
        val result = JSObject()
        result.put("granted", granted())
        invoke.resolve(result)
    }

    @Command
    fun request(invoke: Invoke) {
        when {
            granted() -> answer(invoke)
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.R -> {
                // a page of the system settings and not a dialog, the answer is read once it is closed
                val intent = Intent(
                    Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:${host.packageName}"),
                )
                startActivityForResult(invoke, intent, "settingsClosed")
            }
            else -> requestPermissionForAlias("legacy", invoke, "answered")
        }
    }

    @ActivityCallback
    private fun settingsClosed(invoke: Invoke, @Suppress("UNUSED_PARAMETER") result: ActivityResult) = answer(invoke)

    @PermissionCallback
    private fun answered(invoke: Invoke) = answer(invoke)
}
