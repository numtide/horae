# Temporary dependency patches

## dioxus-fullstack 0.7.9

The workspace patches only this crate, for both the native server and WASM client.
It does not upgrade the rest of Dioxus or change its public API.

Source: the published [0.7.9 crate](https://static.crates.io/crates/dioxus-fullstack/dioxus-fullstack-0.7.9.crate),
SHA-256 `37f0558edb88af5ad47275ae36a7f06317163ba482db377c26d7d8590b5cd0f6`.
The normalized manifest, original manifest, VCS metadata, sources, tests and README
are preserved. The unused upstream lockfile and editor settings are omitted.
`LICENSE-MIT` and `LICENSE-APACHE` come from the upstream `v0.7.9` tag; this copy
is used under the MIT option of the upstream dual license.

The only source change is in `src/magic.rs`:

```diff
-                        let bytes = res.bytes().await.unwrap();
+                        let bytes = res.bytes().await?;
```

This backports [Dioxus PR #5595](https://github.com/DioxusLabs/dioxus/pull/5595),
merge commit `c64415c08f7cef3c26cb8d3ab33985a258476a44`, which fixes
[issue #5585](https://github.com/DioxusLabs/dioxus/issues/5585). A response can
arrive successfully and still fail while its body is read. Propagating that
`RequestError` lets Horae display the failure and retry instead of panicking the
WASM application. No request retry policy is added to Dioxus.

Regression: `crates/horae/tests/browser/new-project-transport.cjs`, run through
`run-design-checks.sh` with its disposable database. It interrupts a response
after the server commits a draft, then checks the visible error, exact request
retry, preservation of newer edits, navigation/recovery and absence of panics.

Remove the Cargo patch, workspace exclusion, vendored directory and its treefmt
exclusion together when the pinned compatible Dioxus release includes this fix.
Regenerate `Cargo.lock` and keep the transport regression in the browser suite.
Do not remove the patch merely because a newer release exists: verify the fix
and run the regression against both the server and WASM build first.
