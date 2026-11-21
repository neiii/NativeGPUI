# The `faim` branch

FAIM's GUI draws its macOS chrome with native-gpui on gpui-fast (neiii/faim-http,
ADR-0017). This branch is Cafekery/NativeGPUI's `main` at 63a384c with the changes
below, and nothing else. FAIM pins it by `rev`.

## Changes

1. **gpui-fast in place of zed's GPUI.** `gpui` is `gpui-fast` 0.1.17, exact, with
   default features off, so it brings no Wayland or X11. The examples'
   `gpui_platform` is `gpui-fast-platform` 0.1.17 with `font-kit`. An app that depends
   on native-gpui turns `font-kit` on through its own `gpui_platform`, as FAIM does.
2. **No vendored `gpui_macos`.** native-gpui's source never names it: the dependency
   only made Cargo build the patched copy in `crates/gpui-macos`. That copy, its seven
   patches and the `[patch]` that pointed zed's GPUI at it are left out. The patches
   are written against zed's `gpui_macos` at 90d8514, before zed moved it from `cocoa`
   to `objc2`, so none applies to gpui-fast's. Porting them to gpui-fast is FAIM's
   follow-up work, on neiii/gpui-fast.
3. **Only native-gpui is in the workspace.** `crates/` (the vendored `gpui_macos`) and
   `founder/` (Found'er) still build against zed's GPUI, so `[workspace]` excludes
   them.
4. **`Keystroke::parse_with_os` for menu chords.** gpui-fast's `Keystroke::parse` takes
   a `&'static str`, and a menu's chord is a `String` built at run time.
   `src/bridge/menus.rs` parses it with `parse_with_os(chord, OperatingSystem::Mac)`,
   the same parser with `secondary-` resolved for macOS.

5. **`image` without its default features.** native-gpui uses its pixel types alone
   (`RgbaImage`, `Frame`). The defaults turn on every codec for gpui-fast's copy too,
   AVIF encoding included, which brings `rav1e` and the unmaintained `paste`
   (RUSTSEC-2024-0436).
6. **No hand-written `IntoElement` for an `Element`.** gpui-fast 0.1.17 syncs zed at
   `78f5d74`, whose GPUI has `impl<T: Element> IntoElement for T`. native-gpui's identity
   impls (`NativeMaterial`, `GlassContainer`, `NativeContextMenu`, `CursorRegion`, the
   window-chrome regions, and the ones `hosted_element!` and `wrapper_element!` write)
   conflict with it and are gone; the blanket impl does the same.

## Checked

On macOS 27 (Apple silicon), `cargo build --examples` builds all 23 examples. Dev
builds need LLVM, since Cranelift can't compile gpui-fast's `ctor` `asm!`.
`window_chrome` and `sidebar` render as on zed's GPUI.

## Updating

Rebase onto Cafekery/NativeGPUI's `main`, keep the six changes above, and move
`gpui-fast` and `gpui-fast-platform` together to the release FAIM's `gui/Cargo.toml`
pins.
