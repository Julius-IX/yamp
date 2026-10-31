package com.salad.yamp.ytdlp

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import kotlin.concurrent.thread

@InvokeArg
class Opt {
    lateinit var k: String
    var v: String = ""
}

@InvokeArg
class RunArgs {
    lateinit var target: String
    var opts: Array<Opt> = arrayOf()
    var channel: Channel? = null
}

@TauriPlugin
class YtdlpPlugin(private val activity: Activity) : Plugin(activity) {
    private val ctx get() = activity.applicationContext
    private var ready = false

    // init unpacks the embedded Python on first use, so it must run once and off the main thread
    @Synchronized
    private fun ensure() {
        if (!ready) {
            YoutubeDL.getInstance().init(ctx)
            ready = true
        }
    }

    private fun bg(invoke: Invoke, f: () -> JSObject) {
        thread {
            try {
                invoke.resolve(f())
            } catch (e: Throwable) {
                invoke.reject(e.message ?: e.toString())
            }
        }
    }

    @Command
    fun version(invoke: Invoke) {
        bg(invoke) {
            ensure()
            JSObject().put("out", YoutubeDL.getInstance().version(ctx) ?: "")
        }
    }

    @Command
    fun update(invoke: Invoke) {
        bg(invoke) {
            ensure()
            YoutubeDL.getInstance().updateYoutubeDL(ctx, YoutubeDL.UpdateChannel.STABLE)
            JSObject().put("out", YoutubeDL.getInstance().version(ctx) ?: "")
        }
    }

    @Command
    fun execute(invoke: Invoke) {
        val a = invoke.parseArgs(RunArgs::class.java)
        bg(invoke) {
            ensure()
            val req = YoutubeDLRequest(a.target)
            a.opts.forEach { if (it.v.isEmpty()) req.addOption(it.k) else req.addOption(it.k, it.v) }
            // the library only reports lines in yt-dlp's default progress format, as a percentage
            val onProgress = a.channel?.let { ch ->
                { percent: Float, _: Long, _: String ->
                    ch.send(JSObject().put("percent", percent.toDouble()))
                }
            }
            val res = if (onProgress != null) {
                YoutubeDL.getInstance().execute(req, callback = onProgress)
            } else {
                YoutubeDL.getInstance().execute(req)
            }
            JSObject().put("out", res.out)
        }
    }
}
