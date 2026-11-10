# Testing yamp

For building and running the app see [BUILDING.md](BUILDING.md).

## What is tested

Tests are unit tests next to the code, in `#[cfg(test)]` modules. There are no integration test
folders and the frontend has no test runner. The frontend is checked by the compiler:

```sh
npx tsc --noEmit
```

## Which crates to test

Run tests per package from [`src-tauri`](../src-tauri), and not for the whole workspace. `cargo test --workspace`
also builds the app and the crates that depend on Tauri, which needs its system libraries and takes
a long time.

These crates have no Tauri dependency, so a test builds only the crate and its own dependencies:

| Crate | Source |
| ----- | ------ |
| `cloud-auth` | [`crates/cloud_auth`](../src-tauri/crates/cloud_auth) |
| `cloud-kit` | [`crates/cloud_kit`](../src-tauri/crates/cloud_kit) |
| `cloud-sync` | [`crates/cloud_sync`](../src-tauri/crates/cloud_sync) |
| `file-server` | [`crates/file_server`](../src-tauri/crates/file_server) |
| `folder-watch` | [`crates/folder_watch`](../src-tauri/crates/folder_watch) |
| `ts-types` | [`crates/ts_types`](../src-tauri/crates/ts_types) |
| `yamp-db` | [`crates/yamp_db`](../src-tauri/crates/yamp_db) |
| `yamp-discord` | [`crates/yamp_discord`](../src-tauri/crates/yamp_discord) |
| `yamp-media` | [`crates/yamp_media`](../src-tauri/crates/yamp_media) |
| `ytm-catalog` | [`crates/ytm_catalog`](../src-tauri/crates/ytm_catalog) |

```sh
cd src-tauri
cargo test -p yamp-db
cargo test -p yamp-discord
```

[`tauri-groups`](../src-tauri/crates/tauri_groups), [`tauri-settings`](../src-tauri/crates/tauri_settings),
[`yamp-session`](../src-tauri/crates/yamp_session), [`yamp-storage`](../src-tauri/crates/yamp_storage),
[`yt-api`](../src-tauri/crates/yt_api) and the app itself need the Tauri prerequisites of the
platform, see [BUILDING.md](BUILDING.md#prerequisites).

No test needs the internet, a Discord client or a cloud account, and no build time key, such as a
Discord application id or a client id, is needed. A few tests of the file server and the sign in
open a port on `127.0.0.1`.

## The database

[`yamp-db`](../src-tauri/crates/yamp_db/README.md#schema) checks every query at compile time. The tests themselves run on an in-memory database,
`Db::memory()`, which is migrated from [`src-tauri/migrations`](../src-tauri/migrations) like a real one. So a test run needs
a database for the compiler only, and there are two ways to give it one. Run from `src-tauri`.

Without a database, from the committed query data:

```sh
export SQLX_OFFLINE=true
cargo test -p yamp-db
```

With a development database, which is needed after changing a query or a migration:

```sh
cargo install sqlx-cli --no-default-features --features sqlite

mkdir -p dev
sqlx database setup          # creates dev/dev-db.sqlite from src-tauri/.env and applies migrations
cargo test -p yamp-db
cargo sqlx prepare --workspace   # rewrites .sqlx, commit the result
```

`DATABASE_URL` comes from [`src-tauri/.env`](../src-tauri/.env). To use another file, export it, for example
`sqlite:target/dev.db`. The folder `dev` is ignored by git.

Other commands, all from `src-tauri`:

```sh
sqlx database reset -y                   # drops the database and applies everything again
cargo sqlx prepare --workspace --check   # fails when .sqlx is out of date, for CI
```

A migration file is never edited after a release, a change is a new file at the end of
[`src-tauri/migrations`](../src-tauri/migrations) with the next number, written by hand and not
with `sqlx migrate add`. After adding one, reset the development database, refresh
[`.sqlx`](../src-tauri/.sqlx) and commit it together with the migration. A stale `.sqlx` shows up as
a build error about a missing query. The rules for the schema are in
[`yamp-db`](../src-tauri/crates/yamp_db/README.md#changing-a-query-or-the-schema).

The migration tests in [`migrate.rs`](../src-tauri/crates/yamp_db/src/migrate.rs) open a database at an older version with `Db::memory_at` and
check that the later files apply on top of it.

## Recommended tooling

- [`cargo-nextest`](https://nexte.st) runs each test in its own process and shows failures more
  clearly than `cargo test`.

  ```sh
  cargo install cargo-nextest --locked
  cargo nextest run -p yamp-db
  cargo nextest run -p yamp-db -p yamp-media -p cloud-kit
  cargo nextest --workspace
  ```

  It does not run doctests, so add `cargo test --doc -p <crate>` where a crate has them.
- `cargo clippy -p <crate> --all-targets` and `cargo fmt --check` before a commit.
- [`bacon`](https://dystroy.org/bacon/) or `cargo watch -x "nextest run -p yamp-db"` for a loop
  that reruns tests on a save.
- [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) to see which code the tests miss.

## Writing a test

- Put it in a `#[cfg(test)] mod tests` at the bottom of the file it covers.
- A test that needs a library takes `Db::memory().await`, and one of a stored format opens the
  database at the version before the change with `Db::memory_at`.
- A test that touches the disk makes its own folder, as the tests of [`file-server`](../src-tauri/crates/file_server/src/lib.rs) do, and cleans
  it up.
- Keep a test off the internet. A parser is tested against a small piece of the response it reads.
