use super::find_output;
use crate::harness::Compositor;

#[test]
fn position_right_of_align_bottom() {
    let mut comp = Compositor::new();
    comp.add_output();
    comp.add_output();

    let out = comp.run_wltile(&["position", "HEADLESS-2", "right-of", "HEADLESS-1"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    let h2 = find_output(&outputs, "HEADLESS-2");

    assert_eq!(h2.rect.x, h1.rect.x + h1.rect.width, "h2 should start where h1 ends");
    assert_eq!(
        h2.rect.y + h2.rect.height,
        h1.rect.y + h1.rect.height,
        "align-bottom: bottom edges should align",
    );
}

#[test]
fn position_right_of_align_top() {
    let mut comp = Compositor::new();
    comp.add_output();
    comp.add_output();

    let out = comp.run_wltile(&["position", "HEADLESS-2", "right-of", "HEADLESS-1", "align-top"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    let h2 = find_output(&outputs, "HEADLESS-2");

    assert_eq!(h2.rect.x, h1.rect.x + h1.rect.width);
    assert_eq!(h2.rect.y, h1.rect.y, "align-top: top edges should align");
}

#[test]
fn position_left_of_align_bottom() {
    let mut comp = Compositor::new();
    comp.add_output();
    comp.add_output();

    let out = comp.run_wltile(&["position", "HEADLESS-2", "left-of", "HEADLESS-1"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    let h2 = find_output(&outputs, "HEADLESS-2");

    assert_eq!(h2.rect.x + h2.rect.width, h1.rect.x, "h2 right edge should touch h1 left edge");
    assert_eq!(h2.rect.y + h2.rect.height, h1.rect.y + h1.rect.height);
}

#[test]
fn position_top_of_align_left() {
    let mut comp = Compositor::new();
    comp.add_output();
    comp.add_output();

    let out = comp.run_wltile(&["position", "HEADLESS-2", "top-of", "HEADLESS-1", "align-left"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    let h2 = find_output(&outputs, "HEADLESS-2");

    assert_eq!(h2.rect.y + h2.rect.height, h1.rect.y, "h2 bottom edge should touch h1 top edge");
    assert_eq!(h2.rect.x, h1.rect.x, "align-left: left edges should align");
}

#[test]
fn position_bottom_of_align_left() {
    let mut comp = Compositor::new();
    comp.add_output();
    comp.add_output();

    let out = comp.run_wltile(&["position", "HEADLESS-2", "bottom-of", "HEADLESS-1", "align-left"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    let h2 = find_output(&outputs, "HEADLESS-2");

    assert_eq!(h2.rect.y, h1.rect.y + h1.rect.height, "h2 top edge should touch h1 bottom edge");
    assert_eq!(h2.rect.x, h1.rect.x);
}

#[test]
fn position_unknown_target_fails() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["position", "nonexistent", "right-of", "HEADLESS-1"]);
    assert!(!out.status.success());
}

#[test]
fn position_unknown_reference_fails() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["position", "HEADLESS-1", "right-of", "nonexistent"]);
    assert!(!out.status.success());
}
