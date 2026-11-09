package com.salad.yamp

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import com.salad.yamp.session.SessionHost

class MainActivity : TauriActivity() {
  private var webView: WebView? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
  }

  // The activity pauses its webview when it leaves the screen, and a paused one does not run the
  // script that moves on to the next track. While music plays it is resumed again right away.
  override fun onPause() {
    super.onPause()
    if (SessionHost.isPlaying) webView?.onResume()
  }
}
