package com.salad.yamp.storage

import android.Manifest
import android.app.Activity
import android.os.Build
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@TauriPlugin(
    permissions = [
        Permission(strings = [Manifest.permission.READ_MEDIA_AUDIO], alias = "audio"),
        Permission(strings = [Manifest.permission.READ_EXTERNAL_STORAGE], alias = "legacy"),
    ]
)
class StoragePlugin(activity: Activity) : Plugin(activity) {
    // the permission that matters depends on the version, the other one is never granted
    private fun alias() = if (Build.VERSION.SDK_INT >= 33) "audio" else "legacy"

    private fun granted() = getPermissionState(alias()) == PermissionState.GRANTED

    private fun answer(invoke: Invoke) {
        val result = JSObject()
        result.put("granted", granted())
        invoke.resolve(result)
    }

    @Command
    fun request(invoke: Invoke) {
        if (granted()) answer(invoke) else requestPermissionForAlias(alias(), invoke, "answered")
    }

    @PermissionCallback
    private fun answered(invoke: Invoke) = answer(invoke)
}
