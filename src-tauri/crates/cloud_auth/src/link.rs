use std::sync::{
  Mutex, PoisonError,
  mpsc::{self, Receiver, Sender},
};

use crate::flow::parse_query;

/// The one sign in that is waiting for a link. A sign in is started by a person, so there is at
/// most one, and a newer one replaces an older one.
struct Waiting {
  /// What the link has to start with, the redirect of the sign in.
  prefix: String,
  state: String,
  tx: Sender<String>,
}

static WAITING: Mutex<Option<Waiting>> = Mutex::new(None);

fn lock() -> std::sync::MutexGuard<'static, Option<Waiting>> {
  WAITING.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn register(prefix: &str, state: &str) -> Receiver<String> {
  let (tx, rx) = mpsc::channel();
  // the receiver of a replaced sign in sees the channel close and gives up
  *lock() = Some(Waiting { prefix: prefix.to_owned(), state: state.to_owned(), tx });
  rx
}

pub(crate) fn clear() {
  *lock() = None;
}

/// Hands a link that the operating system opened the app with to the sign in that waits for it.
/// The app calls this for every such link, and what is not the answer to the sign in is ignored.
/// Returns whether the link was taken.
pub fn deliver_redirect(url: &str) -> bool {
  let guard = lock();
  let Some(waiting) = guard.as_ref() else { return false };
  if !url.starts_with(&waiting.prefix) {
    return false;
  }
  // the state ties the answer to this sign in, a link from anywhere else could carry any code
  let query = url.split_once('?').map_or("", |(_, q)| q);
  if !parse_query(query).iter().any(|(k, v)| k == "state" && *v == waiting.state) {
    return false;
  }
  waiting.tx.send(url.to_owned()).is_ok()
}
