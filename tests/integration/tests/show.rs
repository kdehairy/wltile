use crate::harness::Compositor;

#[test]
fn show_output_prints_details() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let out = comp.run_wltile(&["show", h1.as_str()]);

    assert!(
        out.status.success(),
        "expected `show {h1}` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
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
        assert!(
            stdout.contains(expected),
            "expected `show` output to contain {expected:?}, got:\n{stdout}",
        );
    }
    assert!(
        stdout.contains("> 0."),
        "expected `show` output's Modes list to mark mode 0 as current with \"> 0.\", got:\n{stdout}",
    );
}

#[test]
fn show_output_targets_correct_output_among_multiple() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let out = comp.run_wltile(&["show", h2.as_str()]);

    assert!(
        out.status.success(),
        "expected `show {h2}` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let h2_name_line = format!("Name: {h2}");
    let h1_name_line = format!("Name: {h1}");
    assert!(
        stdout.contains(&h2_name_line),
        "expected `show {h2}` output to contain {h2_name_line:?}, got:\n{stdout}",
    );
    assert!(
        !stdout.contains(&h1_name_line),
        "expected `show {h2}` output to NOT mention {h1_name_line:?} (wrong output targeted), got:\n{stdout}",
    );
}

#[test]
fn show_output_reflects_scale_set_via_set_command() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let set_out = comp.run_wltile(&["set", h1.as_str(), "scale", "2"]);
    assert!(
        set_out.status.success(),
        "expected `set {h1} scale 2` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&set_out.stderr),
    );

    let out = comp.run_wltile(&["show", h1.as_str()]);
    assert!(
        out.status.success(),
        "expected `show {h1}` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Scale: 2"),
        "expected `show` output to contain \"Scale: 2\" after setting {h1}'s scale, got:\n{stdout}",
    );
}

#[test]
fn show_output_reflects_position_after_positioning() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let pos_out = comp.run_wltile(&["position", h2.as_str(), "right-of", h1.as_str(), "align-top"]);
    assert!(
        pos_out.status.success(),
        "expected `position {h2} right-of {h1} align-top` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&pos_out.stderr),
    );

    let outputs = comp.outputs();
    let h2_out = super::find_output(&outputs, &h2);
    let h2_position = format!("Position: ({}, {})", h2_out.rect.x, h2_out.rect.y);

    let out = comp.run_wltile(&["show", h2.as_str()]);
    assert!(
        out.status.success(),
        "expected `show {h2}` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&h2_position),
        "expected `show` output to contain {h2_position:?} ({h2}'s real position per the compositor), got:\n{stdout}",
    );
}

#[test]
fn show_output_prints_disabled_head_correctly() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    comp.disable_output(&h1);

    let out = comp.run_wltile(&["show", h1.as_str()]);

    assert!(
        out.status.success(),
        "expected `show {h1}` to succeed even though it's disabled, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let disabled_name_line = format!("Name: {h1} (disabled)");
    assert!(
        stdout.contains(&disabled_name_line),
        "expected `show` output to contain {disabled_name_line:?}, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Size: N/A"),
        "expected `show` output to show \"Size: N/A\" for the disabled, mode-less {h1}, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Refresh Rate: N/A"),
        "expected `show` output to show \"Refresh Rate: N/A\" for the disabled, mode-less {h1}, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Modes:"),
        "expected `show` output to still contain a \"Modes:\" section, got:\n{stdout}",
    );
}

#[test]
fn show_unknown_output_fails() {
    let mut comp = Compositor::new();
    let _h1 = comp.add_output();

    let out = comp.run_wltile(&["show", "nonexistent"]);
    assert!(
        !out.status.success(),
        "expected `show nonexistent` to fail for a nonexistent output, but it succeeded with stdout:\n{}",
        String::from_utf8_lossy(&out.stdout),
    );
}
