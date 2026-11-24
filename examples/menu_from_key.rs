//! A chooser whose AppKit menu opens from a key as well as a click: ↩ opens it under the
//! chooser through `open_menu`, ⇧↩ opens a menu of `MenuCommand`s through
//! `open_command_menu`, and a click opens the same menu as ↩ through `native_context_menu`.
//! Each pick prints a line.
//!
//! `NG_DRIVE=1` drives it without a person: it sends ↩, picks the second row of the menu
//! that opens, sends ⇧↩, picks the second command, and exits with the number of checks
//! that failed.

use std::cell::{Cell, RefCell};
use std::env;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    App, AsyncApp, Bounds, Context, FocusHandle, KeyBinding, Pixels, Point, Render, Window,
    WindowBounds, WindowHandle, actions, div, point, prelude::*, px, rgb, size,
};
use native_gpui::{
    MenuCommand, MenuEntry, MenuOrigin, native_context_menu, native_root, open_command_menu,
    open_menu, window_options,
};
use objc2::MainThreadOnly;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSApplication, NSEvent, NSEventModifierFlags, NSEventTrackingRunLoopMode, NSEventType, NSMenu,
    NSMenuDidBeginTrackingNotification, NSView, NSWindow,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSPoint, NSProcessInfo, NSRunLoop, NSString, NSTimer,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

actions!(menu_from_key, [OpenHellos, OpenCommands, Curl, Firefox]);

const HELLOS: [&str; 3] = ["Safari's, as #1263 sent it", "curl's", "FAIM's own"];

/// Where the chooser sits in the window, so ↩ opens the menu where a click would.
const CHOOSER: Point<Pixels> = point(px(24.), px(24.));

fn hello_entries(chosen: usize) -> Vec<MenuEntry> {
    HELLOS
        .iter()
        .enumerate()
        .map(|(index, hello)| MenuEntry::item(*hello).checked(index == chosen))
        .collect()
}

struct Chooser {
    chosen: Rc<Cell<usize>>,
    focus: FocusHandle,
}

impl Chooser {
    fn open_hellos(&mut self, _: &OpenHellos, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = self.chosen.clone();
        open_menu(
            CHOOSER,
            hello_entries(chosen.get()),
            move |index, _, _| {
                chosen.set(*index);
                println!("picked hello {index}");
            },
            window,
            cx,
        );
    }

    fn open_commands(&mut self, _: &OpenCommands, window: &mut Window, cx: &mut Context<Self>) {
        open_command_menu(
            CHOOSER,
            [
                MenuCommand::new("Send as curl", Curl),
                MenuCommand::new("Send as Firefox", Firefox),
            ],
            window,
            cx,
        );
    }
}

impl Render for Chooser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focus.is_focused(window) {
            self.focus.focus(window, cx);
        }
        let chosen = self.chosen.clone();
        let row = div()
            .w(px(280.))
            .h(px(24.))
            .px_2()
            .rounded_md()
            .bg(rgb(0x2c2c2e))
            .flex()
            .items_center()
            .child(format!("Hello: {}", HELLOS[self.chosen.get()]));
        native_root(
            div()
                .track_focus(&self.focus)
                .key_context("Chooser")
                .on_action(cx.listener(Self::open_hellos))
                .on_action(cx.listener(Self::open_commands))
                .size_full()
                .bg(rgb(0x1c1c1e))
                .text_color(rgb(0xf2f2f7))
                .child(
                    div().absolute().left(CHOOSER.x).top(CHOOSER.y).child(
                        native_context_menu("hello", row)
                            .opens_on_click()
                            .origin(MenuOrigin::ElementTopLeft)
                            .entries(hello_entries(self.chosen.get()))
                            .on_select(move |index, _, _| {
                                chosen.set(*index);
                                println!("picked hello {index}");
                            }),
                    ),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(24.))
                        .top(px(64.))
                        .child("↩ chooses a hello · ⇧↩ sends as · click opens it too"),
                ),
        )
    }
}

thread_local! {
    static DISPATCHED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static OPENED: Cell<u32> = const { Cell::new(0) };
}

fn ns_window(cx: &mut App, handle: &WindowHandle<Chooser>) -> Option<Retained<NSWindow>> {
    handle
        .update(cx, |_, window, _| {
            let handle = HasWindowHandle::window_handle(window).ok()?;
            let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
                return None;
            };
            let view = unsafe { Retained::retain(appkit.ns_view.as_ptr().cast::<NSView>()) }?;
            view.window()
        })
        .ok()
        .flatten()
}

/// ↩ with `flags` held, sent to `window` as the keyboard would.
fn send_return(window: &NSWindow, flags: NSEventModifierFlags) {
    let text = NSString::from_str("\r");
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        NSEventType::KeyDown,
        NSPoint::ZERO,
        flags,
        NSProcessInfo::processInfo().systemUptime(),
        window.windowNumber(),
        None,
        &text,
        &text,
        false,
        36,
    );
    if let Some(event) = event {
        NSApplication::sharedApplication(window.mtm()).sendEvent(&event);
    }
}

/// Each menu that begins tracking chooses its second row as a person would, then closes.
fn pick_second_row_of_each_menu() {
    let center = NSNotificationCenter::defaultCenter();
    let block = block2::RcBlock::new(|note: std::ptr::NonNull<NSNotification>| {
        let note = unsafe { note.as_ref() };
        let Some(menu) = note
            .object()
            .and_then(|menu| menu.downcast::<NSMenu>().ok())
        else {
            return;
        };
        OPENED.set(OPENED.get() + 1);
        // Chosen from a timer in the tracking loop's own mode, as a click lands mid-tracking.
        let timer = block2::RcBlock::new(move |_: std::ptr::NonNull<NSTimer>| {
            menu.performActionForItemAtIndex(1);
            menu.cancelTracking();
        });
        let timer = unsafe { NSTimer::timerWithTimeInterval_repeats_block(0.3, false, &timer) };
        unsafe { NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSEventTrackingRunLoopMode) };
    });
    let observer = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSMenuDidBeginTrackingNotification),
            None::<&AnyObject>,
            None,
            &block,
        )
    };
    std::mem::forget(observer);
}

fn verdict(failed: &mut u32, name: &str, ok: bool, detail: String) {
    if !ok {
        *failed += 1;
    }
    println!(
        "{:<6} {name:<32} {detail}",
        if ok { "pass" } else { "FAIL" }
    );
}

async fn drive(cx: &mut AsyncApp, handle: WindowHandle<Chooser>, chosen: Rc<Cell<usize>>) {
    let settle = async |cx: &mut AsyncApp| {
        cx.background_executor()
            .timer(Duration::from_millis(800))
            .await;
    };
    settle(cx).await;
    let Some(window) = cx.update(|cx| ns_window(cx, &handle)) else {
        println!("FAIL no window");
        std::process::exit(1);
    };
    window.makeKeyAndOrderFront(None);
    pick_second_row_of_each_menu();
    let mut failed = 0;

    send_return(&window, NSEventModifierFlags::empty());
    settle(cx).await;
    verdict(
        &mut failed,
        "return.opened_a_menu",
        OPENED.get() == 1,
        format!("menus opened {}", OPENED.get()),
    );
    verdict(
        &mut failed,
        "return.picked_the_second_hello",
        chosen.get() == 1,
        format!("chosen {}", chosen.get()),
    );

    send_return(&window, NSEventModifierFlags::Shift);
    settle(cx).await;
    let dispatched = DISPATCHED.with_borrow(Clone::clone);
    verdict(
        &mut failed,
        "shift_return.opened_a_menu",
        OPENED.get() == 2,
        format!("menus opened {}", OPENED.get()),
    );
    verdict(
        &mut failed,
        "shift_return.dispatched_firefox",
        dispatched == ["Firefox"],
        format!("dispatched {dispatched:?}"),
    );

    println!("\n{failed} failed");
    std::process::exit(failed as i32);
}

fn record(name: &'static str) {
    println!("dispatched {name}");
    DISPATCHED.with_borrow_mut(|dispatched| dispatched.push(name));
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("enter", OpenHellos, Some("Chooser")),
            KeyBinding::new("shift-enter", OpenCommands, Some("Chooser")),
        ]);
        cx.on_action(|_: &Curl, _| record("curl"));
        cx.on_action(|_: &Firefox, _| record("Firefox"));

        let chosen = Rc::new(Cell::new(0));
        let handle = cx
            .open_window(
                gpui::WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: point(px(120.), px(120.)),
                        size: size(px(420.), px(200.)),
                    })),
                    ..window_options()
                },
                |_, cx| {
                    let chosen = chosen.clone();
                    cx.new(|cx| Chooser {
                        chosen,
                        focus: cx.focus_handle(),
                    })
                },
            )
            .expect("the example needs a window");
        cx.activate(true);
        if env::var("NG_DRIVE").as_deref() == Ok("1") {
            cx.spawn(async move |cx: &mut AsyncApp| drive(cx, handle, chosen).await)
                .detach();
        }
    });
}
