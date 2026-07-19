use super::find_output;
use crate::harness::Compositor;

#[test]
fn position_right_of_align_bottom() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let out = comp.run_wltile(&["position", h2.as_str(), "right-of", h1.as_str()]);
    assert!(
        out.status.success(),
        "expected `position {h2} right-of {h1}` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    let h2 = find_output(&outputs, &h2);

    assert_eq!(
        h2.rect.x,
        h1.rect.x + h1.rect.width,
        "expected h2's left edge (x={}) to start where h1 ends (x={}), h2 should start where h1 ends",
        h2.rect.x,
        h1.rect.x + h1.rect.width,
    );
    assert_eq!(
        h2.rect.y + h2.rect.height,
        h1.rect.y + h1.rect.height,
        "align-bottom: expected h2's bottom edge ({}) to align with h1's bottom edge ({})",
        h2.rect.y + h2.rect.height,
        h1.rect.y + h1.rect.height,
    );
}

#[test]
fn position_right_of_align_top() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let out = comp.run_wltile(&["position", h2.as_str(), "right-of", h1.as_str(), "align-top"]);
    assert!(
        out.status.success(),
        "expected `position {h2} right-of {h1} align-top` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    let h2 = find_output(&outputs, &h2);

    assert_eq!(
        h2.rect.x,
        h1.rect.x + h1.rect.width,
        "expected h2's left edge (x={}) to start where h1 ends (x={})",
        h2.rect.x,
        h1.rect.x + h1.rect.width,
    );
    assert_eq!(
        h2.rect.y, h1.rect.y,
        "align-top: expected h2's top edge ({}) to align with h1's top edge ({})",
        h2.rect.y, h1.rect.y,
    );
}

#[test]
fn position_left_of_align_bottom() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let out = comp.run_wltile(&["position", h2.as_str(), "left-of", h1.as_str()]);
    assert!(
        out.status.success(),
        "expected `position {h2} left-of {h1}` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    let h2 = find_output(&outputs, &h2);

    assert_eq!(
        h2.rect.x + h2.rect.width,
        h1.rect.x,
        "expected h2's right edge ({}) to touch h1's left edge ({})",
        h2.rect.x + h2.rect.width,
        h1.rect.x,
    );
    assert_eq!(
        h2.rect.y + h2.rect.height,
        h1.rect.y + h1.rect.height,
        "align-bottom: expected h2's bottom edge ({}) to align with h1's bottom edge ({})",
        h2.rect.y + h2.rect.height,
        h1.rect.y + h1.rect.height,
    );
}

#[test]
fn position_top_of_align_left() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let out = comp.run_wltile(&["position", h2.as_str(), "top-of", h1.as_str(), "align-left"]);
    assert!(
        out.status.success(),
        "expected `position {h2} top-of {h1} align-left` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    let h2 = find_output(&outputs, &h2);

    assert_eq!(
        h2.rect.y + h2.rect.height,
        h1.rect.y,
        "expected h2's bottom edge ({}) to touch h1's top edge ({})",
        h2.rect.y + h2.rect.height,
        h1.rect.y,
    );
    assert_eq!(
        h2.rect.x, h1.rect.x,
        "align-left: expected h2's left edge ({}) to align with h1's left edge ({})",
        h2.rect.x, h1.rect.x,
    );
}

#[test]
fn position_bottom_of_align_left() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let out = comp.run_wltile(&["position", h2.as_str(), "bottom-of", h1.as_str(), "align-left"]);
    assert!(
        out.status.success(),
        "expected `position {h2} bottom-of {h1} align-left` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    let h2 = find_output(&outputs, &h2);

    assert_eq!(
        h2.rect.y,
        h1.rect.y + h1.rect.height,
        "expected h2's top edge ({}) to touch h1's bottom edge ({})",
        h2.rect.y,
        h1.rect.y + h1.rect.height,
    );
    assert_eq!(
        h2.rect.x, h1.rect.x,
        "align-left: expected h2's left edge ({}) to align with h1's left edge ({})",
        h2.rect.x, h1.rect.x,
    );
}

#[test]
fn position_unknown_target_fails() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let out = comp.run_wltile(&["position", "nonexistent", "right-of", h1.as_str()]);
    assert!(
        !out.status.success(),
        "expected `position nonexistent right-of {h1}` to fail for an unknown target, but it succeeded with stdout:\n{}",
        String::from_utf8_lossy(&out.stdout),
    );
}

#[test]
fn position_unknown_reference_fails() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let out = comp.run_wltile(&["position", h1.as_str(), "right-of", "nonexistent"]);
    assert!(
        !out.status.success(),
        "expected `position {h1} right-of nonexistent` to fail for an unknown reference, but it succeeded with stdout:\n{}",
        String::from_utf8_lossy(&out.stdout),
    );
}
