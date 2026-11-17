//! A text field's drag past its scroll viewport's edge keeps the viewport scrolling while the
//! press stays there, and the selection follows the content it brings in: down and up, faster
//! with more overshoot, and stopped by a pointer back inside, the press ending, or a cancel.
use super::*;

const LINES: usize = 400;
const LINE_H: f32 = 20.0;
const VIEW_H: f32 = 200.0;
/// Bytes a line of the document takes, its break included.
const LINE_LEN: usize = "line 000 words here\n".len();

struct Doc {
    edit: crate::TextEdit,
}

fn doc_text() -> String { (0..LINES).map(|i| format!("line {i:03} words here")).collect::<Vec<_>>().join("\n") }

fn byte_of_line(line: usize) -> usize { line * LINE_LEN }

fn view(s: &Doc, _: &mut Cx<Doc>) -> El {
    let field = crate::text_input(&s.edit, crate::TextStyle::new(14.0, 400.0).lh(LINE_H))
        .key("doc")
        .w_full()
        .on_edit(|s: &mut Doc| &mut s.edit);
    col().key("root").w(300.0).h(VIEW_H).child(
        col().key("scroll").w_full().h(VIEW_H).scroll(ScrollSpec { stick_bottom: false }).child(field),
    )
}

fn runtime() -> Runtime<Doc> {
    let mut rt = Runtime::new(Doc { edit: crate::TextEdit::new(doc_text()) }, view, (300.0, VIEW_H), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn offset(rt: &Runtime<Doc>) -> f64 { rt.scrolls.values().next().expect("one scroll").offset }

fn max(rt: &Runtime<Doc>) -> f64 { rt.scrolls.values().next().expect("one scroll").max }

fn frames(rt: &mut Runtime<Doc>, n: usize) { for _ in 0..n { rt.frame(1.0 / 60.0); } }

/// Frames until nothing moves and no wake is left.
fn settle(rt: &mut Runtime<Doc>) {
    for _ in 0..240 {
        if !rt.frame(1.0 / 60.0) && rt.wake_deadlines.is_empty() { return; }
    }
    panic!("it settles: {:?}", rt.wake_deadlines);
}

/// Presses at the top of the document and holds the pointer `past` points below the viewport
/// for `n` frames, stationary.
fn hold_below(past: f32, n: usize) -> Runtime<Doc> {
    let mut rt = runtime();
    rt.pointer_down(10.0, 10.0);
    rt.frame(1.0 / 60.0);
    rt.pointer_move(10.0, VIEW_H + past);
    rt.frame(1.0 / 60.0);
    frames(&mut rt, n);
    rt
}

#[test]
fn a_held_drag_past_the_viewport_bottom_keeps_scrolling_and_selecting() {
    let rt = hold_below(20.0, 30);
    assert!(offset(&rt) > 80.0, "the viewport scrolled: {}", offset(&rt));
    assert_eq!(rt.state.edit.anchor, 2, "the anchor stays where the press landed");
    assert!(
        rt.state.edit.focus > byte_of_line(13),
        "the selection reaches past the first viewport: byte {}",
        rt.state.edit.focus,
    );
}

#[test]
fn a_pointer_further_past_the_edge_scrolls_faster() {
    let near = hold_below(20.0, 10);
    let far = hold_below(300.0, 10);
    assert!(offset(&far) > 4.0 * offset(&near), "near {} far {}", offset(&near), offset(&far));
    assert!(far.state.edit.focus > near.state.edit.focus, "and selects further");
}

#[test]
fn a_held_drag_past_the_viewport_top_scrolls_back_up() {
    let mut rt = runtime();
    rt.wheel(150.0, 100.0, -1000.0);
    settle(&mut rt);
    let start = offset(&rt);
    assert!(start > 500.0, "the wheel scrolled down first: {start}");
    rt.pointer_down(10.0, 100.0);
    rt.frame(1.0 / 60.0);
    let anchor = rt.state.edit.anchor;
    rt.pointer_move(10.0, -50.0);
    frames(&mut rt, 20);
    assert!(offset(&rt) < start - 100.0, "it scrolled up: {} from {start}", offset(&rt));
    assert!(rt.state.edit.focus < anchor, "the selection reaches above the press: {} vs {anchor}", rt.state.edit.focus);
}

#[test]
fn holding_past_the_bottom_reaches_the_end_and_stops_there() {
    let mut rt = hold_below(300.0, 400);
    assert_eq!(offset(&rt), max(&rt), "it reached the bottom");
    // The last step's drag lands in the frame after it, as a field's selection is the app's.
    frames(&mut rt, 2);
    assert!(
        rt.state.edit.focus >= byte_of_line(LINES - 1),
        "the selection reaches the last line: byte {}",
        rt.state.edit.focus,
    );
    frames(&mut rt, 5);
    assert!(rt.wake_deadlines.is_empty(), "and it asks for no more frames");
    assert_eq!(offset(&rt), max(&rt));
}

#[test]
fn a_pointer_back_inside_release_and_cancel_stop_the_scroll() {
    let mut rt = hold_below(20.0, 10);
    assert!(offset(&rt) > 0.0, "it was scrolling");
    rt.pointer_move(10.0, 100.0);
    frames(&mut rt, 10);
    let stopped = offset(&rt);
    frames(&mut rt, 10);
    assert_eq!(offset(&rt), stopped, "a pointer inside stops the step");
    rt.pointer_up(10.0, 100.0);
    settle(&mut rt);
    assert!(rt.wake_deadlines.is_empty(), "the release leaves no wake");
    assert_eq!(offset(&rt), stopped, "and the viewport stays put");

    let mut rt = hold_below(20.0, 10);
    rt.cancel_pointer();
    settle(&mut rt);
    let cancelled = offset(&rt);
    frames(&mut rt, 10);
    assert_eq!(offset(&rt), cancelled, "a cancelled press stops the step");
    assert!(rt.wake_deadlines.is_empty(), "and leaves no wake");
}
