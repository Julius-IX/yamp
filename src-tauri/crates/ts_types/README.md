# ts-types

Generates TypeScript declarations from Rust types.

## Usage

Derive `TsType` on a struct or a unit-only enum:

```rust
use ts_types::TsType;

#[derive(serde::Serialize, TsType)]
#[serde(rename_all = "camelCase")]
struct Book {
  title: String,
  page_count: Option<u32>,
}
```

Write the declarations with `render_types_ts()`, which returns the content of `types.ts`.

## Attributes

Reads `#[serde(rename_all, rename, skip)]`. Extra attributes:

| Attribute | Effect |
| --------- | ------ |
| `#[ts(type = "string")]` | Overrides the TS type of a field |
| `#[ts(skip)]` | Leaves a field out |

## Limits

- Generic types and enums with data are not derived. Implement `TsType` by hand.
- `#[serde(flatten)]` is a compile error.
- 64 and 128 bit integers map to `number`.

## Features

| Feature | Adds impls for |
| ------- | -------------- |
| `chrono` | `DateTime`, `NaiveDate`, `NaiveTime`, `NaiveDateTime` |
| `uuid` | `Uuid` |

## Layout

The `macros` directory is the proc macro crate. It is re-exported and not used directly.
