use super::find_output;
use crate::harness::Compositor;

#[test]
fn set_scale_is_reflected_in_compositor() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let out = comp.run_wltile(&["set", h1.as_str(), "scale", "2"]);
    assert!(
        out.status.success(),
        "expected `set {h1} scale 2` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    assert!(
        (h1.scale - 2.0).abs() < 0.01,
        "expected the compositor to report the output's scale as 2.0, got {}",
        h1.scale,
    );
}

#[test]
fn set_active_false_disables_output_in_compositor() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let out = comp.run_wltile(&["set", h1.as_str(), "active", "false"]);
    assert!(
        out.status.success(),
        "expected `set {h1} active false` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    assert!(
        !h1.active,
        "expected the compositor to report the output as inactive after `set active false`",
    );
}

#[test]
fn set_active_true_reenables_disabled_output_in_compositor() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    comp.disable_output(&h1);

    let out = comp.run_wltile(&["set", h1.as_str(), "active", "true"]);
    assert!(
        out.status.success(),
        "expected `set {h1} active true` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, &h1);
    assert!(
        h1.active,
        "expected the compositor to report the output as active after `set active true`",
    );
}
