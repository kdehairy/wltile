use super::find_output;
use crate::harness::Compositor;

#[test]
fn set_scale_is_reflected_in_compositor() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["set", "HEADLESS-1", "scale", "2"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    assert!((h1.scale - 2.0).abs() < 0.01, "expected scale 2.0, got {}", h1.scale);
}
