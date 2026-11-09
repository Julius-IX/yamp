package com.salad.yamp.session

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder

/**
 * Exists only to keep the process alive and the notification pinned while music plays in the
 * background. The session and the notification are owned by [SessionHost].
 */
class PlaybackService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        instance = this
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // a press on a notification button comes through here as well
        intent?.action?.let { SessionHost.dispatchButton(it) }
        val notification = SessionHost.notification
        if (notification == null) {
            stopSelf()
            return START_NOT_STICKY
        }
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(SessionHost.NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(SessionHost.NOTIFICATION_ID, notification)
        }
        pinned = true
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        instance = null
        pinned = false
        super.onDestroy()
    }

    companion object {
        var instance: PlaybackService? = null

        /** Whether the service is in the foreground. A start from the background is refused on newer Android, so this is checked before asking for one. */
        @Volatile
        var pinned = false
    }
}
