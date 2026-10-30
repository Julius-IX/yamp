# tauri-groups

Grouped Tauri commands with generated TypeScript bindings.

## Usage

Put a command in a group:

```rust
#[tauri_groups::command(books)]
fn list(db: State<Db>, query: Option<String>) -> Result<Vec<Book>, String> { .. }
```

Inline modules can use `#[group]` and a bare `#[command]` instead:

```rust
#[tauri_groups::group]
mod books {
  #[command]
  fn list() -> Vec<Book> { .. }
}
```

Build the handler from the registry:

```rust
tauri::Builder::default().invoke_handler(tauri_groups::handler())
```

Write `commands.ts` and `types.ts`:

```rust
tauri_groups::export_ts("src/generated")?;
```

## Naming

A command is registered as `<group>__<fn>`, so `files::list` and `db::list` can coexist. Names must be unique within a group.

## Types

Argument and return types need `TsType`, which is re-exported from `ts-types`. The root crate must also depend on `ts-types` because the derive expands to its paths.

## Limits

- `#[group]` only works on inline modules. Use `#[command(group)]` on each fn for modules in their own file.
- Arguments injected by Tauri are left out of the signature.

## Features

`chrono` and `uuid` are forwarded to `ts-types`.

## Layout

The `macros` directory is the proc macro crate. It is re-exported and not used directly.
