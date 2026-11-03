// `sqlx::migrate!` and the query macros read files and the environment without telling
// cargo, so a change to them would not rebuild the crate.
fn main() {
  println!("cargo:rerun-if-changed=../../migrations");
  println!("cargo:rerun-if-changed=../../.sqlx");
  println!("cargo:rerun-if-env-changed=DATABASE_URL");
  println!("cargo:rerun-if-env-changed=SQLX_OFFLINE");
}
