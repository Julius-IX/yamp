package com.salad.yamp

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import com.salad.yamp.session.SessionHost

class MainActivity : TauriActivity() {
  private var webView: WebView? = null

  // The frontend owns the navigation stacks, so the default back handling of the webview history
  // is turned off. Nothing is pushed to that history anymore.
  override val handleBackNavigation = false

  private val backCallback = object : OnBackPressedCallback(true) {
    override fun handleOnBackPressed() {
      val view = webView ?: return
      view.evaluateJavascript("window.yampBack ? window.yampBack() : false") { handled ->
        if (handled == "true") return@evaluateJavascript
        // nothing left in the frontend to go back to, so the press goes on to the default
        isEnabled = false
        onBackPressedDispatcher.onBackPressed()
        isEnabled = true
      }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
    onBackPressedDispatcher.addCallback(this, backCallback)
  }

  // The activity pauses its webview when it leaves the screen, and a paused one does not run the
  // script that moves on to the next track. While music plays it is resumed again right away.
  override fun onPause() {
    super.onPause()
    if (SessionHost.isPlaying) webView?.onResume()
  }
}
