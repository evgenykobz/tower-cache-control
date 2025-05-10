*Tower* layer that simplify setting `Cache-Control` response header, featuring:
- Opinionated `Cache-Control` value based on the response status
- Customizable default value

---

## Installation

```toml
[dependencies]
tower-cache-control = "1.0.0"
```
## Usage

Layer `CacheControlLayer` comes with a default value (via `Default` trait),
although it supports a custom `CacheControl` setting (via `axum-extra` crate re-export).
