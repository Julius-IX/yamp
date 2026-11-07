package com.salad.yamp.session

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.os.Build
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class ListenArgs {
    lateinit var channel: Channel
}

@InvokeArg
class UpdateArgs {
    var title: String? = null
    var artist: String = ""
    var album: String? = null
    var cover: String? = null
    var durationSecs: Double? = null
    var positionSecs: Double = 0.0
    var playing: Boolean = false
}

@TauriPlugin
class SessionPlugin(private val activity: Activity) : Plugin(activity) {
    private var asked = false

    @Command
    fun listen(invoke: Invoke) {
        val channel = invoke.parseArgs(ListenArgs::class.java).channel
        SessionHost.attach(activity.applicationContext)
        SessionHost.onAction = { action -> channel.send(action) }
        invoke.resolve()
    }

    @Command
    fun update(invoke: Invoke) {
        val a = invoke.parseArgs(UpdateArgs::class.java)
        SessionHost.attach(activity.applicationContext)
        if (a.title != null) askForNotifications()
        SessionHost.update(a)
        invoke.resolve()
    }

    // The notification is silently dropped without this on Android 13 and newer. Asked once,
    // the first time something plays, so the prompt has a reason the person can see.
    private fun askForNotifications() {
        if (asked || Build.VERSION.SDK_INT < 33) return
        asked = true
        val permission = Manifest.permission.POST_NOTIFICATIONS
        if (activity.checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
            activity.runOnUiThread { activity.requestPermissions(arrayOf(permission), 7) }
        }
    }
}
