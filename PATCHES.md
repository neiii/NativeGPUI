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
7. **No toolbar or window-tab rows in the menu bar.** `install_menus` leaves out New Tab
   (File), Show Toolbar, Customize Toolbar…, Show Tab Bar and Show All Tabs (View), and
   Show Previous Tab, Show Next Tab, Move Tab to New Window and Merge All Windows
   (Window). FAIM's windows have no toolbar and no tabs, and gpui turns automatic window
   tabbing off for a window opened without a tabbing identifier, so AppKit adds none of
   its own either. `window_tabs` still gives a window tabs, with its + button, which
   `install_menus` still teaches the app delegate; an app that calls it adds the rows it
   wants with `extend_menu`. The `menu_bar` probe checks no menu holds one.
8. **`NativeLabel::fitting_size(window)`.** The size AppKit fits a label's text to on one
   line, at the font the label draws with, rounded up to `window`'s pixels as a drawn
   field's `fittingSize` is. It measures on every call, with no cache to grow by every
   text an app shows. FAIM's toasts size their capsule to it. The `label_fitting`
   probe checks it against the drawn label's own `fittingSize`, and that a box one point
   narrower cuts the text off.
9. **A menu opened from a key.** `open_menu(at, entries, on_close, window, cx)` opens
   `MenuEntry` rows as an AppKit menu with its top-left corner at a point in the window,
   where a click puts a `native_context_menu` with `MenuOrigin::ElementTopLeft`, and runs
   `on_close` once it closes, with the chosen row's index or `None` when Esc or a click
   outside closed it, so an app can take away what it marked while the menu was open
   (FAIM's row menu rings the row it is open on, #909); and
   `open_command_menu(at, commands, window, cx)` does the same for `MenuCommand` rows,
   each dispatching its action as a menu bar row does; the rows a popup is opened with
   are released from the action table once it closes, where the menu bar's stay (rows a
   filled submenu adds while it is open stay, as the menu bar's filled submenus' do). Both enter AppKit's
   tracking loop on a later turn through `track_later`, which `NativeContextMenu`'s click
   path was rewritten to share, so a rebase can conflict in its `paint` too.
   The `menu_from_key` example opens one on ↩ and the other on ⇧↩; with `NG_DRIVE=1` it
   sends the keys, picks a row in each and checks what it got, then closes a third with
   no pick and checks `on_close` got `None`.
10. **`MenuCommand::boxed(title, action)`.** A row from an action an app holds as a
   `Box<dyn Action>`, which `new`'s `impl Action` can't take: FAIM builds its menu bar
   from a table of commands, each making its action boxed. `new` calls it.
11. **`Glass::passes_presses()`.** A glass that lets a press through to whatever is under
   it: FAIM's toast is Liquid Glass at a pane's foot and asks nothing of the pointer, so
   a click on the rows under it has to reach them. Stock, `NSGlassEffectView` answers
   `hitTest:` with itself across its bounds; with `passes_presses` the glass is an
   `NGPassingGlass`, which answers with nothing, as `NGBackdrop` does for a material. A
   glass whose setting changes is rebuilt, as the host rebuilds any slot whose view
   changes type.

## Checked

On macOS 27 (Apple silicon), `cargo build --examples` builds every example and probe,
with and without `private-api`. Dev
builds need LLVM, since Cranelift can't compile gpui-fast's `ctor` `asm!`.
`window_chrome` and `sidebar` render as on zed's GPUI. Changes 7 to 9 were checked in a
macOS 27 VM: the `menu_bar` probe (its `fire` rows stay BLOCKED unbundled), the
`label_fitting` probe, and `menu_from_key` with `NG_DRIVE=1`. Two `cargo test --lib`
tests, `the_folder_icon_is_blue` and `the_installer_package_icon_is_orange`, fail on
macOS 27 before these changes too.

## Updating

Rebase onto Cafekery/NativeGPUI's `main`, keep the changes above, and move
`gpui-fast` and `gpui-fast-platform` together to the release FAIM's `gui/Cargo.toml`
pins.
