//! Gate: `NativeLabel::fitting_size` is the size AppKit fits the drawn label to in its window,
//! so a box of that size shows the whole text. Prints each label's size; exits 0 pass,
//! non-zero fail.

use std::time::Duration;

use gpui::{
    App, AsyncApp, Bounds, Context, FontWeight, Pixels, Render, Size, Window, WindowBounds,
    WindowHandle, div, point, prelude::*, px, size,
};
use native_gpui::{NativeLabel, native_label, native_root, window_options};
use objc2::rc::Retained;
use objc2_app_kit::{NSTextField, NSView, NSWindow};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

struct Case {
    text: &'static str,
    points: f32,
    weight: FontWeight,
    tabular: bool,
    /// Drawn this much narrower than its fitting width: the instrument must call it cut off.
    short_by: f32,
}

const fn case(text: &'static str, points: f32, weight: FontWeight, tabular: bool) -> Case {
    Case {
        text,
        points,
        weight,
        tabular,
        short_by: 0.,
    }
}

const CASES: &[Case] = &[
    case("Replay sent", 13., FontWeight::NORMAL, false),
    case(
        "#1283 sent with curl's hello",
        13.,
        FontWeight::MEDIUM,
        false,
    ),
    case("Wide WWWWWW", 15., FontWeight::BOLD, false),
    case("narrow iiiiii", 11., FontWeight::NORMAL, false),
    case("1111 vs 0000", 13., FontWeight::NORMAL, true),
    Case {
        short_by: 1.,
        ..case(
            "One point short of its width",
            13.,
            FontWeight::MEDIUM,
            false,
        )
    },
];

impl Case {
    fn label(&self) -> NativeLabel {
        native_label(self.text, self.text)
            .font_size(px(self.points))
            .weight(self.weight)
            .tabular(self.tabular)
    }

    fn fit(&self, window: &Window) -> Size<Pixels> {
        self.label().fitting_size(window)
    }

    fn drawn(&self, window: &Window) -> Size<Pixels> {
        let fit = self.fit(window);
        size(fit.width - px(self.short_by), fit.height)
    }
}

struct Probe;

impl Render for Probe {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let column = div().flex().flex_col().items_start().gap_2().p_2();
        native_root(column.children(CASES.iter().map(|case| {
            let drawn = case.drawn(window);
            div()
                .w(drawn.width)
                .h(drawn.height)
                .flex_none()
                .child(case.label())
        })))
    }
}

fn ns_window(cx: &mut App, handle: &WindowHandle<Probe>) -> Option<Retained<NSWindow>> {
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

fn fields(view: &NSView, found: &mut Vec<Retained<NSTextField>>) {
    for child in view.subviews().iter() {
        if let Ok(field) = child.clone().downcast::<NSTextField>() {
            found.push(field);
        }
        fields(&child, found);
    }
}

/// AppKit's own answer to "is this cut off": a non-empty expansion frame.
fn cut_off(field: &NSTextField) -> bool {
    let Some(cell) = field.cell() else {
        return true;
    };
    let frame = cell.expansionFrameWithFrame_inView(field.bounds(), field);
    frame.size.width > 0. || frame.size.height > 0.
}

fn check(window: &NSWindow, fits: &[Size<Pixels>]) -> u32 {
    let Some(root) = window
        .contentView()
        .and_then(|view| unsafe { view.superview() })
    else {
        println!("FAIL no view tree");
        return 1;
    };
    let mut found = Vec::new();
    fields(&root, &mut found);
    println!(
        "{:<30} {:>8} {:>8} {:>8} {:>8}  VERDICT",
        "TEXT", "FIT.W", "APPKIT.W", "FIT.H", "APPKIT.H"
    );
    let mut failed = 0;
    for (case, &fit) in CASES.iter().zip(fits) {
        let Some(field) = found
            .iter()
            .find(|field| field.stringValue().to_string() == case.text)
        else {
            println!("{:<30} no drawn label  WRONG", case.text);
            failed += 1;
            continue;
        };
        let appkit = field.fittingSize();
        let same = (f64::from(f32::from(fit.width)) - appkit.width).abs() < 0.01
            && (f64::from(f32::from(fit.height)) - appkit.height).abs() < 0.01;
        let cut = cut_off(field);
        let ok = same && cut == (case.short_by > 0.);
        if !ok {
            failed += 1;
        }
        println!(
            "{:<30} {:>8.2} {:>8.2} {:>8.2} {:>8.2}  {} {}",
            case.text,
            f32::from(fit.width),
            appkit.width,
            f32::from(fit.height),
            appkit.height,
            if cut { "cut off" } else { "whole" },
            if ok { "ok" } else { "WRONG" }
        );
    }
    failed
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let handle = cx
            .open_window(
                gpui::WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: point(px(40.), px(40.)),
                        size: size(px(420.), px(260.)),
                    })),
                    ..window_options()
                },
                |_, cx| cx.new(|_| Probe),
            )
            .expect("the probe needs a window to draw its labels in");

        cx.spawn(async move |cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(Duration::from_millis(600))
                .await;
            let fits = handle.update(cx, |_, window, _| {
                CASES
                    .iter()
                    .map(|case| case.fit(window))
                    .collect::<Vec<_>>()
            });
            let ns = cx.update(|cx| ns_window(cx, &handle));
            let failed = match (ns, fits) {
                (Some(ns), Ok(fits)) => check(&ns, &fits),
                _ => 1,
            };
            println!("\n{failed} failed");
            std::process::exit(failed.min(120) as i32);
        })
        .detach();
    });
}
