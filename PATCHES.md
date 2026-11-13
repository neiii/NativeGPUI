# The `faim` branch

FAIM's GUI draws its macOS chrome with native-gpui on gpui-fast (neiii/faim-http,
ADR-0017). This branch is Cafekery/NativeGPUI's `main` at d78415b with the changes
below, and nothing else. FAIM pins it by `rev`.

## Changes

1. **gpui-fast in place of zed's GPUI.** `gpui` is `gpui-fast` 0.1.14, exact, with
   default features off, so it brings no Wayland or X11. The examples'
   `gpui_platform` is `gpui-fast-platform` 0.1.14 with `font-kit`. An app that depends
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

## Checked

On macOS 27 (Apple silicon), `cargo build --examples` builds all 23 examples. Dev
builds need LLVM, since Cranelift can't compile gpui-fast's `ctor` `asm!`.
`window_chrome` and `sidebar` render as on zed's GPUI.

## Updating

Rebase onto Cafekery/NativeGPUI's `main`, keep the four changes above, and move
`gpui-fast` and `gpui-fast-platform` together to the release FAIM's `gui/Cargo.toml`
pins.
