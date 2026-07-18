use super::find_output;
use crate::harness::Compositor;

#[test]
fn set_scale_is_reflected_in_compositor() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["set", "HEADLESS-1", "scale", "2"]);
    assert!(
        out.status.success(),
        "expected `set HEADLESS-1 scale 2` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    assert!(
        (h1.scale - 2.0).abs() < 0.01,
        "expected the compositor to report HEADLESS-1's scale as 2.0, got {}",
        h1.scale,
    );
}

#[test]
fn set_active_false_disables_output_in_compositor() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2

    let out = comp.run_wltile(&["set", "HEADLESS-2", "active", "false"]);
    assert!(
        out.status.success(),
        "expected `set HEADLESS-2 active false` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h2 = find_output(&outputs, "HEADLESS-2");
    assert!(
        !h2.active,
        "expected the compositor to report HEADLESS-2 as inactive after `set active false`",
    );
}

#[test]
fn set_active_true_reenables_disabled_output_in_compositor() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2
    comp.disable_output("HEADLESS-2");

    let out = comp.run_wltile(&["set", "HEADLESS-2", "active", "true"]);
    assert!(
        out.status.success(),
        "expected `set HEADLESS-2 active true` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );

    let outputs = comp.outputs();
    let h2 = find_output(&outputs, "HEADLESS-2");
    assert!(
        h2.active,
        "expected the compositor to report HEADLESS-2 as active after `set active true`",
    );
}
