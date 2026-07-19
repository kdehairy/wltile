use crate::harness::Compositor;

#[test]
fn list_shows_all_connected_outputs() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let out = comp.run_wltile(&["list"]);

    assert!(
        out.status.success(),
        "expected `list` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(h1.as_str()),
        "expected `list` output to contain {h1:?}, got:\n{stdout}",
    );
    assert!(
        stdout.contains(h2.as_str()),
        "expected `list` output to contain {h2:?}, got:\n{stdout}",
    );
}

#[test]
fn list_reports_all_expected_fields() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let out = comp.run_wltile(&["list"]);

    assert!(
        out.status.success(),
        "expected `list` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for expected in [
        h1.as_str(),
        "Serial Number:",
        "Make:",
        "Size:",
        "scale:",
        "Physical Size:",
        "Refresh Rate:",
        "Position:",
    ] {
        assert!(
            stdout.contains(expected),
            "expected `list` output to contain {expected:?}, got:\n{stdout}",
        );
    }
}

#[test]
fn list_reflects_scale_set_via_set_command() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let set_out = comp.run_wltile(&["set", h1.as_str(), "scale", "2"]);
    assert!(
        set_out.status.success(),
        "expected `set {h1} scale 2` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&set_out.stderr),
    );

    let out = comp.run_wltile(&["list"]);
    assert!(
        out.status.success(),
        "expected `list` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("scale: 2"),
        "expected `list` output to contain \"scale: 2\" after setting {h1}'s scale, got:\n{stdout}",
    );
}

#[test]
fn list_reflects_position_after_positioning() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let pos_out = comp.run_wltile(&[
        "position",
        h2.as_str(),
        "right-of",
        h1.as_str(),
        "align-top",
    ]);
    assert!(
        pos_out.status.success(),
        "expected `position {h2} right-of {h1} align-top` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&pos_out.stderr),
    );

    let outputs = comp.outputs();
    let h1_out = super::find_output(&outputs, &h1);
    let h2_out = super::find_output(&outputs, &h2);
    let h1_position = format!("Position: ({}, {})", h1_out.rect.x, h1_out.rect.y);
    let h2_position = format!("Position: ({}, {})", h2_out.rect.x, h2_out.rect.y);

    let out = comp.run_wltile(&["list"]);
    assert!(
        out.status.success(),
        "expected `list` to succeed, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&h1_position),
        "expected `list` output to contain {h1_position:?} ({h1}'s real position per the compositor), got:\n{stdout}",
    );
    assert!(
        stdout.contains(&h2_position),
        "expected `list` output to contain {h2_position:?} ({h2}'s real position per the compositor), got:\n{stdout}",
    );
}

#[test]
fn list_shows_disabled_output() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();
    comp.disable_output(&h2);

    let out = comp.run_wltile(&["list"]);
    assert!(
        out.status.success(),
        "expected `list` to succeed even with a disabled output present, but it failed with stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(h1.as_str()),
        "expected `list` output to still contain the enabled {h1:?}, got:\n{stdout}",
    );
    let disabled = format!("{h2} (disabled)");
    assert!(
        stdout.contains(&disabled),
        "expected `list` output to contain {disabled:?}, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Size: N/A"),
        "expected `list` output to show \"Size: N/A\" for the disabled, mode-less {h2}, got:\n{stdout}",
    );
    assert!(
        stdout.contains("Refresh Rate: N/A"),
        "expected `list` output to show \"Refresh Rate: N/A\" for the disabled, mode-less {h2}, got:\n{stdout}",
    );
}
