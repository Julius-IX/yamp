//! Small key value pairs that are shared between the devices of the account. The settings that
//! follow the user to another device and the listening statistics are kept here. What a value
//! holds is for the frontend to decide, see `yamp-db`.
use super::{fail, wire::StoredValue};
use tauri::State;
use yamp_db::Db;

/// Every pair whose key starts with the prefix, in key order.
#[tauri_groups::command(values)]
pub async fn list(db: State<'_, Db>, prefix: String) -> Result<Vec<StoredValue>, String> {
  let found = db.values().list(&prefix).await.map_err(fail)?;
  Ok(found.into_iter().map(|(key, value)| StoredValue { key, value }).collect())
}

#[tauri_groups::command(values)]
pub async fn set(db: State<'_, Db>, key: String, value: String) -> Result<(), String> {
  db.values().set(&key, &value).await.map_err(fail)
}

/// Returns how many pairs were removed.
#[tauri_groups::command(values)]
pub async fn remove_prefix(db: State<'_, Db>, prefix: String) -> Result<u64, String> {
  db.values().remove_prefix(&prefix).await.map_err(fail)
}
