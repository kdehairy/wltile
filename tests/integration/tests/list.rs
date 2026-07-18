use super::find_output;
use crate::harness::Compositor;

#[test]
fn list_shows_all_connected_outputs() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-1
    comp.add_output(); // HEADLESS-2

    let out = comp.run_wltile(&["list"]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("HEADLESS-1"), "stdout: {stdout}");
    assert!(stdout.contains("HEADLESS-2"), "stdout: {stdout}");
}

#[test]
fn list_reports_all_expected_fields() {
    let comp = Compositor::new(); // relies on the always-present default HEADLESS-1

    let out = comp.run_wltile(&["list"]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for expected in [
        "HEADLESS-1",
        "Serial Number:",
        "Make:",
        "Size:",
        "scale:",
        "Physical Size:",
        "Refresh Rate:",
        "Position:",
    ] {
        assert!(stdout.contains(expected), "expected {expected:?} in stdout: {stdout}");
    }
}

#[test]
fn list_reflects_scale_set_via_set_command() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2

    let set_out = comp.run_wltile(&["set", "HEADLESS-2", "scale", "2"]);
    assert!(set_out.status.success(), "stderr: {}", String::from_utf8_lossy(&set_out.stderr));

    let out = comp.run_wltile(&["list"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("scale: 2"), "stdout: {stdout}");
}

#[test]
fn list_reflects_position_after_positioning() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2

    let pos_out = comp.run_wltile(&["position", "HEADLESS-2", "right-of", "HEADLESS-1", "align-top"]);
    assert!(pos_out.status.success(), "stderr: {}", String::from_utf8_lossy(&pos_out.stderr));

    let outputs = comp.outputs();
    let h1 = find_output(&outputs, "HEADLESS-1");
    let h2 = find_output(&outputs, "HEADLESS-2");
    let h1_position = format!("Position: ({}, {})", h1.rect.x, h1.rect.y);
    let h2_position = format!("Position: ({}, {})", h2.rect.x, h2.rect.y);

    let out = comp.run_wltile(&["list"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&h1_position), "expected {h1_position:?} in stdout: {stdout}");
    assert!(stdout.contains(&h2_position), "expected {h2_position:?} in stdout: {stdout}");
}

#[test]
fn list_omits_disabled_output() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2
    comp.disable_output("HEADLESS-2");

    let out = comp.run_wltile(&["list"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("HEADLESS-1"), "stdout: {stdout}");
    assert!(!stdout.contains("HEADLESS-2"), "stdout: {stdout}");
}
