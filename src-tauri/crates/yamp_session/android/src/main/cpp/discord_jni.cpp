// The Discord presence of Android, shown by the Discord Social SDK through the Discord app of the
// phone. No sign in is involved, the SDK needs only the application id and a Discord app that is
// installed and signed in. The functions are called by `DiscordNative` in Kotlin.
//
// The SDK is not thread safe and wants its callbacks pumped. Both happen on one thread, and the
// JNI functions only leave the wanted state behind for it, which also means that a burst of
// updates, such as one for every seek, costs one call to the SDK.
#include <jni.h>

#include <android/log.h>

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

#define DISCORDPP_IMPLEMENTATION
#include <discordpp.h>

namespace {

constexpr const char* TAG = "yamp-discord";

struct Wanted {
  std::string name, details, state, details_url, state_url, cover, hover, badge, site;
  std::vector<std::pair<std::string, std::string>> buttons;
  uint64_t start_ms = 0;
  uint64_t end_ms = 0;
};

std::mutex lock;
std::condition_variable wake;
std::optional<Wanted> pending;
bool clear_pending = false;
bool started = false;
uint64_t application_id = 0;

std::string str(JNIEnv* env, jstring s) {
  if (!s) return {};
  const char* chars = env->GetStringUTFChars(s, nullptr);
  std::string out(chars);
  env->ReleaseStringUTFChars(s, chars);
  return out;
}

discordpp::Activity build(const Wanted& w) {
  discordpp::Activity activity;
  activity.SetType(discordpp::ActivityTypes::Listening);
  // the name is what the status shows after "Listening to"
  activity.SetName(w.name);
  activity.SetStatusDisplayType(discordpp::StatusDisplayTypes::Name);
  activity.SetDetails(w.details);
  activity.SetState(w.state);
  if (!w.details_url.empty()) activity.SetDetailsUrl(w.details_url);
  if (!w.state_url.empty()) activity.SetStateUrl(w.state_url);

  discordpp::ActivityTimestamps stamps;
  stamps.SetStart(w.start_ms);
  if (w.end_ms > 0) stamps.SetEnd(w.end_ms);
  activity.SetTimestamps(stamps);

  discordpp::ActivityAssets assets;
  if (!w.cover.empty()) assets.SetLargeImage(w.cover);
  assets.SetLargeText(w.hover);
  if (!w.details_url.empty()) assets.SetLargeUrl(w.details_url);
  assets.SetSmallImage(w.badge);
  assets.SetSmallUrl(w.site);
  assets.SetSmallText("yamp");
  activity.SetAssets(assets);

  for (const auto& [label, url] : w.buttons) {
    discordpp::ActivityButton button;
    button.SetLabel(label);
    button.SetUrl(url);
    activity.AddButton(button);
  }
  return activity;
}

void run() {
  auto client = std::make_shared<discordpp::Client>();
  client->SetApplicationId(application_id);
  for (;;) {
    std::optional<Wanted> next;
    bool clear = false;
    {
      std::unique_lock<std::mutex> guard(lock);
      // the callbacks of the SDK are only delivered by `RunCallbacks`, so this cannot sleep for long
      wake.wait_for(guard, std::chrono::milliseconds(100), [] { return pending.has_value() || clear_pending; });
      next = std::move(pending);
      pending.reset();
      clear = clear_pending;
      clear_pending = false;
    }
    if (next) {
      client->UpdateRichPresence(build(*next), [](discordpp::ClientResult result) {
        if (!result.Successful()) {
          __android_log_print(ANDROID_LOG_WARN, TAG, "presence was refused: %s", result.Error().c_str());
        }
      });
    } else if (clear) {
      client->ClearRichPresence();
    }
    discordpp::RunCallbacks();
  }
}

}  // namespace

extern "C" {

JNIEXPORT jboolean JNICALL Java_com_salad_yamp_session_DiscordNative_init(JNIEnv* env, jclass, jstring id) {
  std::lock_guard<std::mutex> guard(lock);
  if (started) return JNI_TRUE;
  try {
    application_id = std::stoull(str(env, id));
  } catch (...) {
    __android_log_print(ANDROID_LOG_WARN, TAG, "the application id is not a number");
    return JNI_FALSE;
  }
  std::thread(run).detach();
  started = true;
  return JNI_TRUE;
}

JNIEXPORT void JNICALL Java_com_salad_yamp_session_DiscordNative_update(
    JNIEnv* env, jclass, jstring name, jstring details, jstring state, jstring details_url, jstring state_url,
    jstring cover, jstring hover, jstring badge, jstring site, jstring button1_label, jstring button1_url,
    jstring button2_label, jstring button2_url, jlong start_ms, jlong end_ms) {
  Wanted w;
  w.name = str(env, name);
  w.details = str(env, details);
  w.state = str(env, state);
  w.details_url = str(env, details_url);
  w.state_url = str(env, state_url);
  w.cover = str(env, cover);
  w.hover = str(env, hover);
  w.badge = str(env, badge);
  w.site = str(env, site);
  for (auto [label, url] : {std::pair{button1_label, button1_url}, std::pair{button2_label, button2_url}}) {
    if (!label || !url) continue;
    auto text = str(env, label);
    if (!text.empty()) w.buttons.emplace_back(text, str(env, url));
  }
  w.start_ms = static_cast<uint64_t>(start_ms);
  w.end_ms = end_ms > 0 ? static_cast<uint64_t>(end_ms) : 0;
  {
    std::lock_guard<std::mutex> guard(lock);
    pending = std::move(w);
    clear_pending = false;
  }
  wake.notify_one();
}

JNIEXPORT void JNICALL Java_com_salad_yamp_session_DiscordNative_clear(JNIEnv*, jclass) {
  {
    std::lock_guard<std::mutex> guard(lock);
    pending.reset();
    clear_pending = true;
  }
  wake.notify_one();
}

}  // extern "C"
