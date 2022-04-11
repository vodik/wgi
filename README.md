# wgi

A weekend experiment crossing old with new technology. Limited error handling,
no focus thoughts on security. Use at your own risk!

An attempt to reimplement CGI using WebAssembly + WASI.

## Examples

### `hello_world.wasm`

Written in C. Reads the user agent header and prints a friendly message.

### `markdown.wasm`

Written in Rust. Converts markdown input into HTML.

### `js.wasm`

Written in C. Embeds [QuickJS][] to bootstrap arbitrary JavaScript files.

## Roadmap

- [ ] Write instructions
- [ ] Cleanup the server and make it robust
- [ ] Implement FastCGI on WASI
- [ ] Implement Lambda on WASI
- [ ] Write blog posts

  [QuickJS]: https://bellard.org/quickjs
