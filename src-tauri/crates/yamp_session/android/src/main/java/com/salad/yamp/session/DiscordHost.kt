package com.salad.yamp.session

import android.app.Activity
import app.tauri.plugin.JSObject

/**
 * The Discord presence of Android. It is shown by the Discord Social SDK, which is a library that
 * is not part of the repository. A build may or may not contain it, so the SDK is reached through
 * reflection and a library that only exists when it was built. A build without it still compiles
 * and reports that the presence is not available. The setup is in the readme of `yamp-discord`.
 *
 * The card is made in Rust, see `Card` in `yamp-discord`, and arrives here finished.
 */
object DiscordHost {
    /** Whether the native library loaded, which it only does in a build that has the SDK. */
    private val usable: Boolean by lazy {
        try {
            System.loadLibrary("yamp_discord")
            true
        } catch (e: UnsatisfiedLinkError) {
            false
        }
    }

    private var started = false

    /** The SDK wants the activity before anything else. A build without the SDK has nothing to give it to. */
    fun engine(activity: Activity) {
        try {
            Class.forName("com.discord.socialsdk.DiscordSocialSdkInit")
                .getMethod("setEngineActivity", Activity::class.java)
                .invoke(null, activity)
        } catch (e: Throwable) {
            // no SDK in this build
        }
    }

    fun run(a: JSObject): JSObject {
        val result = JSObject()
        when (a.optString("op")) {
            "available" -> result.put("available", usable)
            "update" -> if (usable) show(a)
            "clear" -> if (usable && started) DiscordNative.clear()
        }
        return result
    }

    private fun show(a: JSObject) {
        if (!started) started = DiscordNative.init(a.getString("appId"))
        if (!started) return
        val card = a.getJSONObject("card")
        // a field that is absent is an empty one, the native side leaves out what is empty
        fun text(name: String) = if (card.isNull(name)) "" else card.optString(name)
        val buttons = card.optJSONArray("buttons")
        fun button(at: Int, part: Int) = buttons?.optJSONArray(at)?.optString(part) ?: ""
        DiscordNative.update(
            text("name"), text("details"), text("state"), text("detailsUrl"), text("stateUrl"),
            text("cover"), text("hover"), text("badgeUrl"), text("siteUrl"),
            button(0, 0), button(0, 1), button(1, 0), button(1, 1),
            card.optLong("startMs"), card.optLong("endMs", 0),
        )
    }
}

/** The functions of `discord_jni.cpp`. They only exist in a build that has the SDK. */
object DiscordNative {
    @JvmStatic external fun init(appId: String): Boolean

    @JvmStatic external fun update(
        name: String, details: String, state: String, detailsUrl: String, stateUrl: String,
        cover: String, hover: String, badge: String, site: String,
        button1Label: String, button1Url: String, button2Label: String, button2Url: String,
        startMs: Long, endMs: Long,
    )

    @JvmStatic external fun clear()
}
