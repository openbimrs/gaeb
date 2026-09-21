# gaeb

Short-name compatibility package for [`openbim-gaeb`](https://crates.io/crates/openbim-gaeb).
It defines no types and re-exports the canonical implementation wholesale, so
applications using either package share identical Rust types.

```rust
use gaeb::Document;

let document = Document::parse(
    br#"<GAEB xmlns="http://www.gaeb.de/GAEB_DA_XML/DA83/3.3"><Award><DP>83</DP></Award></GAEB>"#,
)?;
assert_eq!(document.metadata().phase.unwrap().as_code(), "83");
# Ok::<(), gaeb::Error>(())
```

## License

MIT
