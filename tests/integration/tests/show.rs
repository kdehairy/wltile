use super::find_output;
use crate::harness::Compositor;

#[test]
fn show_output_prints_details() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["show", "HEADLESS-1"]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for expected in [
        "Name:",
        "Serial Number:",
        "Make:",
        "Model:",
        "Size:",
        "Scale:",
        "Rotation:",
        "Physical Size:",
        "Refresh Rate:",
        "Position:",
        "Modes:",
    ] {
        assert!(stdout.contains(expected), "expected {expected:?} in stdout: {stdout}");
    }
    // the single available mode should be marked as the current one
    assert!(stdout.contains("> 0."), "stdout: {stdout}");
}

#[test]
fn show_output_targets_correct_output_among_multiple() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2

    let out = comp.run_wltile(&["show", "HEADLESS-2"]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Name: HEADLESS-2"), "stdout: {stdout}");
    assert!(!stdout.contains("Name: HEADLESS-1"), "stdout: {stdout}");
}

#[test]
fn show_output_reflects_scale_set_via_set_command() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2

    let set_out = comp.run_wltile(&["set", "HEADLESS-2", "scale", "2"]);
    assert!(set_out.status.success(), "stderr: {}", String::from_utf8_lossy(&set_out.stderr));

    let out = comp.run_wltile(&["show", "HEADLESS-2"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Scale: 2"), "stdout: {stdout}");
}

#[test]
fn show_output_reflects_position_after_positioning() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2

    let pos_out = comp.run_wltile(&["position", "HEADLESS-2", "right-of", "HEADLESS-1", "align-top"]);
    assert!(pos_out.status.success(), "stderr: {}", String::from_utf8_lossy(&pos_out.stderr));

    let outputs = comp.outputs();
    let h2 = find_output(&outputs, "HEADLESS-2");
    let h2_position = format!("Position: ({}, {})", h2.rect.x, h2.rect.y);

    let out = comp.run_wltile(&["show", "HEADLESS-2"]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&h2_position), "expected {h2_position:?} in stdout: {stdout}");
}

#[test]
fn show_output_prints_disabled_head_correctly() {
    let mut comp = Compositor::new();
    comp.add_output(); // HEADLESS-2
    comp.disable_output("HEADLESS-2");

    let out = comp.run_wltile(&["show", "HEADLESS-2"]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Name: HEADLESS-2 (disabled)"), "stdout: {stdout}");
    assert!(stdout.contains("Size: N/A"), "stdout: {stdout}");
    assert!(stdout.contains("Refresh Rate: N/A"), "stdout: {stdout}");
    assert!(stdout.contains("Modes:"), "stdout: {stdout}");
}

#[test]
fn show_unknown_output_fails() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["show", "nonexistent"]);
    assert!(!out.status.success());
}
