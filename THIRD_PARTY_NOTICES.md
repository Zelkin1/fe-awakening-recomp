# Third-party notices

| Component | Where | License | Use |
|---|---|---|---|
| [Zakuro](https://github.com/fearkov/zakuro) | `zakuro/` (a clone), changed by `patches/zakuro-local-changes.patch` | MIT, `zakuro/LICENSE` | The runtime the game runs in |
| [3dsrecomp](https://github.com/fearkov/3dsrecomp) | `3dsrecomp/` (a clone), changed by `patches/3dsrecomp-*.patch` | MIT, as its `Cargo.toml` declares (the repository has no license file) | The recompiler |
| Rust toolchain (rustup) | `tools/rust/`, `tools/dl/rustup-init.exe` | MIT or Apache-2.0 | Builds Zakuro, 3dsrecomp and the port; downloaded, not part of the project |
| Python with NumPy and Pillow | installed separately | BSD-3-Clause (NumPy), MIT-CMU (Pillow) | The scripts in `tools/` |

The crates Zakuro and 3dsrecomp depend on keep their own licenses, listed in
their `Cargo.lock` files.

## Zakuro and 3dsrecomp

The patches carry portions of Zakuro's and 3dsrecomp's code. They are shared,
if at all, under this notice, Zakuro's license file as it reads; 3dsrecomp
declares the same license without a file of its own:

```
MIT License

Copyright (c) 2026 fearkov

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
