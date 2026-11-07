package com.salad.yamp.session

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.drawable.Icon
import android.media.MediaMetadata
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.content.ContextCompat
import app.tauri.plugin.JSObject
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

/**
 * The media session and its notification. A singleton because the service and the plugin both
 * reach it and neither can hold the other. Playback itself stays in the webview, this only
 * mirrors it and passes the presses back as the actions of the Rust side.
 */
object SessionHost {
    const val NOTIFICATION_ID = 41
    private const val CHANNEL_ID = "playback"
    private const val ACTION_PREVIOUS = "com.salad.yamp.session.PREVIOUS"
    private const val ACTION_TOGGLE = "com.salad.yamp.session.TOGGLE"
    private const val ACTION_NEXT = "com.salad.yamp.session.NEXT"
    private const val SEEK_STEP_SECS = 10.0

    /** Sends an action to the frontend. Set once the frontend has started listening. */
    var onAction: ((JSObject) -> Unit)? = null

    /** The notification the service pins. Null while nothing is playing. */
    @Volatile
    var notification: Notification? = null
        private set

    private lateinit var context: Context
    private var session: MediaSession? = null
    private var last: UpdateArgs? = null
    private var coverUrl: String? = null
    private var cover: Bitmap? = null
    private val main = Handler(Looper.getMainLooper())

    @Synchronized
    fun attach(app: Context) {
        if (session != null) return
        context = app.applicationContext
        if (Build.VERSION.SDK_INT >= 26) {
            val channel = NotificationChannel(CHANNEL_ID, "Playback", NotificationManager.IMPORTANCE_LOW)
            channel.setShowBadge(false)
            manager().createNotificationChannel(channel)
        }
        session = MediaSession(context, "yamp").also {
            @Suppress("DEPRECATION")
            it.setFlags(MediaSession.FLAG_HANDLES_MEDIA_BUTTONS or MediaSession.FLAG_HANDLES_TRANSPORT_CONTROLS)
            it.setCallback(callback, main)
        }
    }

    private val callback = object : MediaSession.Callback() {
        override fun onPlay() = send("play")
        override fun onPause() = send("pause")
        override fun onSkipToNext() = send("next")
        override fun onSkipToPrevious() = send("previous")
        override fun onStop() = send("stop")
        override fun onFastForward() = seekBy(SEEK_STEP_SECS)
        override fun onRewind() = seekBy(-SEEK_STEP_SECS)
        override fun onSeekTo(pos: Long) {
            onAction?.invoke(JSObject().put("action", "seekTo").put("secs", pos / 1000.0))
        }
    }

    private fun send(action: String) {
        onAction?.invoke(JSObject().put("action", action))
    }

    private fun seekBy(secs: Double) {
        onAction?.invoke(JSObject().put("action", "seekBy").put("secs", secs))
    }

    /** A press on one of the notification's own buttons. */
    fun dispatchButton(intentAction: String) {
        when (intentAction) {
            ACTION_PREVIOUS -> send("previous")
            ACTION_NEXT -> send("next")
            // the button shows what a press does, so the action follows the state it was drawn for
            ACTION_TOGGLE -> send(if (last?.playing == true) "pause" else "play")
        }
    }

    @Synchronized
    fun update(args: UpdateArgs) {
        val session = session ?: return
        if (args.title == null) {
            clear(session)
            return
        }
        if (args.cover != coverUrl) {
            coverUrl = args.cover
            cover = null
            args.cover?.let { loadCover(it) }
        }
        last = args
        publish(session, args)
    }

    private fun publish(session: MediaSession, args: UpdateArgs) {
        val metadata = MediaMetadata.Builder()
            .putString(MediaMetadata.METADATA_KEY_TITLE, args.title)
            .putString(MediaMetadata.METADATA_KEY_ARTIST, args.artist)
            .putString(MediaMetadata.METADATA_KEY_ALBUM, args.album)
        args.durationSecs?.takeIf { it > 0 }?.let { metadata.putLong(MediaMetadata.METADATA_KEY_DURATION, (it * 1000).toLong()) }
        cover?.let { metadata.putBitmap(MediaMetadata.METADATA_KEY_ALBUM_ART, it) }
        session.setMetadata(metadata.build())

        val actions = PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or PlaybackState.ACTION_PLAY_PAUSE or
            PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_SKIP_TO_PREVIOUS or PlaybackState.ACTION_STOP or
            PlaybackState.ACTION_SEEK_TO or PlaybackState.ACTION_FAST_FORWARD or PlaybackState.ACTION_REWIND
        val state = if (args.playing) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED
        session.setPlaybackState(
            PlaybackState.Builder()
                .setActions(actions)
                .setState(state, (args.positionSecs * 1000).toLong(), if (args.playing) 1f else 0f)
                .build(),
        )
        session.isActive = true

        val built = buildNotification(session, args)
        notification = built
        if (args.playing) {
            try {
                ContextCompat.startForegroundService(context, Intent(context, PlaybackService::class.java))
            } catch (e: Exception) {
                // a start from the background can be refused, the plain notification still works
                manager().notify(NOTIFICATION_ID, built)
            }
        } else {
            // a paused notification can be swiped away, so it is let go of by the service
            PlaybackService.instance?.stopForeground(Service.STOP_FOREGROUND_DETACH)
            manager().notify(NOTIFICATION_ID, built)
        }
    }

    private fun clear(session: MediaSession) {
        last = null
        notification = null
        coverUrl = null
        cover = null
        session.isActive = false
        PlaybackService.instance?.stopSelf()
        manager().cancel(NOTIFICATION_ID)
    }

    private fun buildNotification(session: MediaSession, args: UpdateArgs): Notification {
        val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(context, CHANNEL_ID) else Notification.Builder(context)
        val open = context.packageManager.getLaunchIntentForPackage(context.packageName)?.let {
            PendingIntent.getActivity(context, 0, it, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        }
        return builder
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentTitle(args.title)
            .setContentText(args.artist)
            .setLargeIcon(cover)
            .setContentIntent(open)
            .setOnlyAlertOnce(true)
            .setOngoing(args.playing)
            .setVisibility(Notification.VISIBILITY_PUBLIC)
            .addAction(button(android.R.drawable.ic_media_previous, "Previous", ACTION_PREVIOUS))
            .addAction(
                if (args.playing) button(android.R.drawable.ic_media_pause, "Pause", ACTION_TOGGLE)
                else button(android.R.drawable.ic_media_play, "Play", ACTION_TOGGLE),
            )
            .addAction(button(android.R.drawable.ic_media_next, "Next", ACTION_NEXT))
            .setStyle(Notification.MediaStyle().setMediaSession(session.sessionToken).setShowActionsInCompactView(0, 1, 2))
            .build()
    }

    private fun button(icon: Int, title: String, action: String): Notification.Action {
        val intent = Intent(context, PlaybackService::class.java).setAction(action)
        val pending = PendingIntent.getService(context, action.hashCode(), intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return Notification.Action.Builder(Icon.createWithResource(context, icon), title, pending).build()
    }

    // The cover is not part of the update, so it is fetched here and the session is published again once it is there.
    private fun loadCover(url: String) {
        thread {
            val bitmap = try {
                val connection = URL(url).openConnection() as HttpURLConnection
                connection.connectTimeout = 8000
                connection.readTimeout = 8000
                connection.inputStream.use { BitmapFactory.decodeStream(it) }
            } catch (e: Exception) {
                null
            }
            synchronized(this) {
                // a newer track may have replaced the one this cover was for
                if (bitmap == null || coverUrl != url) return@thread
                cover = bitmap
                val session = session
                val current = last
                if (session != null && current != null) publish(session, current)
            }
        }
    }

    private fun manager() = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
}
