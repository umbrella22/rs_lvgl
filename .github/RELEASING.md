# Releases

All four crates share one version and the tag `vMAJOR.MINOR.PATCH`.
The release workflow publishes `rs_lvgl_codegen` and `rs_lvgl_sys` before
`rs_lvgl`, followed by `rs_lvgl_ui`.

## Credentials

New uploads require a secret named `CARGOTOKEN` in the GitHub Environment `cargo.io`.
Its crates.io API token needs permission to create the four crates on the first
release and publish their subsequent versions. This is the same environment
and secret naming used by Vesper's Rust release workflow; secrets are configured
separately for each repository.

crates.io requires an API token for the first publication. Trusted Publishing
can be configured on each crate after that release.

## Prepare and publish

1. Update all four package versions and their internal dependency versions.
2. Run the normal CI and `python3 scripts/release.py verify --allow-dirty`.
   Verification needs Python 3.12+, Cargo 1.99.1, a C/C++ compiler and libclang.
   Generate the ignored workspace lockfile first if it is absent.
3. Commit and push to `main`. Wait for CI and package verification to pass.
4. Create the matching annotated tag and push it, for example:

   ```sh
   git tag -a v0.2.0 -m "rs_lvgl 0.2.0"
   git push origin v0.2.0
   ```

Tag verification requires a clean checkout at the exact tag commit, reachable
from `origin/main`. The workflow verifies packaged files and extracted consumers
before accessing publication credentials. Native source, headers, build helpers,
licenses and the crate-local examples are included. Root workspace examples are
available from the repository.

The publish job reconciles existing registry versions before uploading any
missing crate. Each downloaded archive must match its crates.io checksum and
the clean tag's source. Generated Cargo lockfiles may differ when re-resolving
dependencies; the original published lockfile remains in the registry archive.
After all four versions are indexed, independent raw and UI applications run
using only crates.io dependencies. GitHub Release is created last.

The [website repository](https://github.com/umbrella22/rs_lvgl_website) builds
and deploys the bilingual guides, WebAssembly studio and editable browser examples
through its own GitHub Pages environment at
[the documentation site](https://umbrella22.github.io/rs_lvgl_website/).
Its `source` submodule pins the documented library revision. Update that revision
and run the website acceptance checks when preparing documentation for a release.
Keep README installation versions and the release's changelog section in sync.
Verify the public guides and playground after the website deployment succeeds;
see [site maintenance](https://github.com/umbrella22/rs_lvgl_website#build-from-a-clean-checkout)
for build and Pages configuration.

## Resume

Run the **Crates release** workflow manually with the existing `release_tag`,
such as `v0.2.0`. It checks out that exact tag; it does not publish newer main
sources under an existing version. Matching published crates are verified and
skipped. A yanked version, different source, registry HTTP failure or missing
upload credential stops publication. A fully published release can be reconciled
without an upload credential. Published versions and tags are never replaced.

```sh
python3 scripts/release.py status v0.2.0
python3 scripts/release.py consumer v0.2.0
```

Artifacts contain archives, checksums, native consumer logs and command receipts.
Token values are supplied through the environment and never command arguments.
