# Third-party research and notices

No third-party implementation source or binary asset is vendored into our source
tree. Ignored `references/` contains local upstream checkouts, each with its own
licensing/provenance. Do not package that directory. Our authored source is
licensed MIT OR Apache-2.0 (LICENSE-MIT, LICENSE-APACHE).

B1 adds Cargo dependencies eframe/egui 0.36.2, its wgpu 30.0.1 backend, and image
0.25.10 (local QA PNG encoding). These crates declare MIT OR Apache-2.0; Cargo.lock
pins their transitive dependency tree. Crate license declarations are not a complete
binary-distribution notice audit. Before any authorized distribution, collect notices
for every resolved dependency/backend. No GUI
crate source or compiled dependency is vendored in the authored tree.

The layout rules used by the independent Python probes and Rust A1 readers were learned
from Craftworld Toolkit/cwlib at d6a2ef80b5a09999f08f23a803b516096055e5c9:
[ennuo/toolkit](https://github.com/ennuo/toolkit), classes listed in docs/formats.md.
Retain this notice for any source adaptation/translation derived from MIT-covered
Toolkit files; separately audit inherited per-file third-party code and assets.

```text
MIT License

Copyright (c) 2024 ennuo

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

Optional synthetic oracle tests execute an external source module from
[Zhaxxy/lbptoolspy-lite](https://github.com/Zhaxxy/lbptoolspy-lite), revision
9e044e0f068d9f3f4db3608e8a4305e1f2f76ce9, GPL-3.0. That module remains in an
ignored independent reference checkout and is not copied into this project's code
or redistributed with it. No encryption/save module is imported for that test.

Other sources were read as documentation/behavioral references; their URLs,
licenses, confidence and permitted research roles are recorded in
docs/community-projects.md and RESEARCH.md. No license grant is inferred for
missing-license repositories or their assets. A top-level open-source license is
not evidence of rights to bundled proprietary/recovered material.
## Texture oracle provenance

Pinned Toolkit's external/DDSReader.java carries MIT, Copyright (c) 2015 Kenji
Sasaki, pointing to https://github.com/npedotnet/DDSReader/blob/master/LICENSE.
It is executed only through the ignored external source-built library, not
vendored or translated. Existing Toolkit notice above remains retained.

Toolkit DDS::convertSwizzleGtf cites RPCS3 rsx_utils.h at
3d49976b3c0f2d2fe5fbd9dba0419c13b389c6ba; that revision's root LICENSE is GPLv2.
Exact contribution/relicensing history is unresolved. Do not infer MIT permission
to translate this helper from Toolkit's root license. No such code is reused.
Refresh's GtfDecoder remains AGPL reference only. See docs/textures.md for links,
the tested external oracle scope and independent specification route for BC1.
