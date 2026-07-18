use crate::harness::Compositor;

#[test]
fn show_output_prints_details() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["show", "HEADLESS-1"]);

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Size:"), "stdout: {stdout}");
    assert!(stdout.contains("Position:"), "stdout: {stdout}");
    assert!(stdout.contains("Modes:"), "stdout: {stdout}");
}

#[test]
fn show_unknown_output_fails() {
    let mut comp = Compositor::new();
    comp.add_output();

    let out = comp.run_wltile(&["show", "nonexistent"]);
    assert!(!out.status.success());
}
