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
