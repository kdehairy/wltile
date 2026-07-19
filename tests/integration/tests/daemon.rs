use std::time::Duration;

use super::find_output;
use crate::harness::Compositor;

const RELOAD_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn daemon_applies_config_on_sighup() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    // 1. Start the daemon with an initial config. Scale 3 is arbitrary but
    // distinguishable from the compositor's default scale (1), so we can
    // tell whether it was actually applied.
    let config_path = comp.write_config(&format!(
        r#"
[{h1}]
scale = 3
"#,
    ));
    let daemon = comp.spawn_daemon(&config_path);

    // 2. The daemon applies the config on startup, without any SIGHUP: its
    // initial Wayland connection sync fires the same update signal used for
    // hotplug/reload, so this should already be true before we ever signal it.
    let initial_applied = comp.wait_for_outputs(RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        (h1.scale - 3.0).abs() < 0.01
    });
    assert!(
        initial_applied,
        "expected the daemon to apply the initial config on startup (no SIGHUP sent yet), \
         outputs: {:?}",
        comp.outputs(),
    );

    // 3. Change the config file on disk, without touching the daemon yet.
    // Deliberately a property-only change (no position): position::exec's
    // fallback re-layout of un-positioned heads reads head geometry from a
    // cache that's shared with, but updated independently of, set_property::exec's
    // just-applied change (they use separate Wayland event queues; the shared
    // cache only catches up via the client's background 500ms poll). Mixing a
    // scale change with a position dependent on that same head's geometry in
    // one reload can race against that cache — a real interaction, but not
    // what this test is about, so it's exercised separately and kept out here.
    comp.write_config(&format!(
        r#"
[{h1}]
scale = 2
"#,
    ));

    // 4 + 5. Only now send SIGHUP, and confirm the *new* config is what gets
    // reflected — proving SIGHUP re-reads the file rather than the daemon
    // just re-applying what it already had from startup.
    let reloaded = comp.reload_until(&daemon, RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        (h1.scale - 2.0).abs() < 0.01
    });
    assert!(
        reloaded,
        "expected the daemon to apply the updated config after SIGHUP, final outputs: {:?}",
        comp.outputs(),
    );
}

#[test]
fn daemon_applies_config_on_output_hotplug() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    // HEADLESS-2 doesn't exist yet — add_output()'s naming is deterministic, so
    // we can reference it in the config ahead of time.
    let h2 = "HEADLESS-2";
    let config_path = comp.write_config(&format!(
        r#"
[{h2}]
position = "right-of {h1} align-bottom"
"#,
    ));

    let _daemon = comp.spawn_daemon(&config_path);

    let created = comp.add_output();
    assert_eq!(
        created, h2,
        "expected the second created output to be named {h2}, got {created}",
    );

    let applied = comp.wait_for_outputs(RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        let h2 = find_output(outputs, h2);
        h2.rect.x == h1.rect.x + h1.rect.width
            && h2.rect.y + h2.rect.height == h1.rect.y + h1.rect.height
    });

    assert!(
        applied,
        "expected the daemon to auto-apply {h2}'s position once it appeared, without any \
         SIGHUP, final outputs: {:?}",
        comp.outputs(),
    );
}

#[test]
fn daemon_relayouts_when_second_output_appears() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    // HEADLESS-2 and HEADLESS-3 don't exist yet — add_output()'s naming is
    // deterministic, so we can reference them in the config ahead of time.
    let h2 = "HEADLESS-2";
    let h3 = "HEADLESS-3";
    let config_path = comp.write_config(&format!(
        r#"
[{h2}]
position = "right-of {h1} align-bottom"

[{h3}]
position = "right-of {h2} align-bottom"
"#,
    ));

    let _daemon = comp.spawn_daemon(&config_path);

    let created_h2 = comp.add_output();
    assert_eq!(
        created_h2, h2,
        "expected the second created output to be named {h2}, got {created_h2}",
    );

    let h2_positioned = comp.wait_for_outputs(RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        let h2 = find_output(outputs, h2);
        h2.rect.x == h1.rect.x + h1.rect.width
            && h2.rect.y + h2.rect.height == h1.rect.y + h1.rect.height
    });
    assert!(
        h2_positioned,
        "expected {h2} to be positioned right-of {h1} after it appeared, outputs: {:?}",
        comp.outputs(),
    );

    let created_h3 = comp.add_output();
    assert_eq!(
        created_h3, h3,
        "expected the third created output to be named {h3}, got {created_h3}",
    );

    let h3_positioned = comp.wait_for_outputs(RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        let h2 = find_output(outputs, h2);
        let h3 = find_output(outputs, h3);
        h2.rect.x == h1.rect.x + h1.rect.width && h3.rect.x == h2.rect.x + h2.rect.width
    });
    assert!(
        h3_positioned,
        "expected {h3} to be positioned right-of {h2} (with {h2} still right-of {h1}) after \
         it appeared, outputs: {:?}",
        comp.outputs(),
    );
}

#[test]
fn daemon_skips_unconnected_targets() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();

    let config_path = comp.write_config(&format!(
        r#"
[{h1}]
scale = 2

[DP-99]
scale = 3
"#,
    ));

    let mut daemon = comp.spawn_daemon(&config_path);

    let applied = comp.wait_for_outputs(RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        (h1.scale - 2.0).abs() < 0.01
    });
    assert!(
        applied,
        "expected the daemon to apply {h1}'s scale despite an unconnected target (DP-99) \
         also being present in the config, outputs: {:?}",
        comp.outputs(),
    );
    assert!(
        daemon.is_alive(),
        "expected the daemon to still be running after applying a config referencing an \
         unconnected target",
    );
}

#[test]
fn daemon_skips_position_with_missing_reference() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let before = comp.outputs();
    let h2_before = find_output(&before, &h2);
    let before_size = (h2_before.rect.width, h2_before.rect.height);

    let config_path = comp.write_config(&format!(
        r#"
[{h1}]
scale = 2

[{h2}]
position = "right-of DP-99 align-bottom"
"#,
    ));

    let _daemon = comp.spawn_daemon(&config_path);

    // Use h1's scale change as our positive control: once it's applied we know
    // the daemon actually processed the config, so any (lack of) effect on h2
    // reflects how its own (invalid) position target was handled, not that
    // the reload never happened.
    let applied = comp.wait_for_outputs(RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        (h1.scale - 2.0).abs() < 0.01
    });
    assert!(
        applied,
        "expected the daemon to apply {h1}'s scale (positive control), outputs: {:?}",
        comp.outputs(),
    );

    let after = comp.outputs();
    let h1_after = find_output(&after, &h1);
    let h2_after = find_output(&after, &h2);

    // h2's own size (scale/mode) can't have changed: its whole target was
    // filtered out of the reload due to the missing reference, so wltile
    // never touched it at all.
    assert_eq!(
        before_size,
        (h2_after.rect.width, h2_after.rect.height),
        "expected {h2}'s size to be untouched since its whole target was filtered out \
         (invalid position reference), outputs: {:?}",
        comp.outputs(),
    );

    // We can't assert h2's absolute position never moved: sway's own default
    // output arrangement may reflow an unpinned, auto-positioned output like
    // h2 when a sibling's size changes (h1's scale, here) — independent of
    // wltile. What we CAN assert is that h2 was never bottom-aligned against
    // h1: that's the specific shape "right-of h1 align-bottom" would produce,
    // which is what we'd see if the missing reference (DP-99) had been
    // incorrectly resolved to h1 instead of being skipped.
    assert_ne!(
        h2_after.rect.y + h2_after.rect.height,
        h1_after.rect.y + h1_after.rect.height,
        "expected {h2} to NOT be bottom-aligned with {h1} — that would indicate the missing \
         reference (DP-99) was resolved to {h1} instead of being skipped, outputs: {:?}",
        comp.outputs(),
    );
}

/// Regression test for a race in `daemon::reload_configs`: it calls
/// `set_property::exec` then immediately `position::exec` in the same pass.
/// `position::exec` reads head geometry (e.g. scale, to compute
/// `scaled_corrected_size`) from the shared `Configurations` cache, which is
/// only refreshed by a background thread polling the main Wayland event
/// queue every ~500ms — decoupled from `set_property::exec`'s own commit,
/// which goes through a separate `ConfigWriter`-owned event queue. So a
/// single reload that both rescales a head and positions another head
/// relative to it can compute the new position using the rescaled head's
/// STALE, pre-change geometry.
///
/// A single SIGHUP (no resend) is used deliberately: resending would let a
/// later, lucky retry paper over the race once the background poll happens
/// to catch up, defeating the point of isolating this one reload's outcome.
#[test]
fn daemon_position_uses_fresh_geometry_after_property_change_in_same_reload() {
    let mut comp = Compositor::new();
    let h1 = comp.add_output();
    let h2 = comp.add_output();

    let config_path = comp.write_config("");
    let daemon = comp.spawn_daemon(&config_path);

    // Give the daemon a moment to finish connecting and registering its
    // signal handlers before we send anything.
    std::thread::sleep(Duration::from_millis(500));

    comp.write_config(&format!(
        r#"
[{h1}]
scale = 2

[{h2}]
position = "right-of {h1} align-bottom"
"#,
    ));
    daemon.reload();

    // A single SIGHUP with a generous timeout — plenty of time for this one
    // reload_configs invocation to fully complete, but no resend, so a lucky
    // later retry can't paper over the race we're isolating.
    let applied = comp.wait_for_outputs(RELOAD_TIMEOUT, |outputs| {
        let h1 = find_output(outputs, &h1);
        let h2 = find_output(outputs, &h2);
        (h1.scale - 2.0).abs() < 0.01
            && h2.rect.x == h1.rect.x + h1.rect.width
            && h2.rect.y + h2.rect.height == h1.rect.y + h1.rect.height
    });

    assert!(
        applied,
        "expected {h2} to be positioned using {h1}'s NEW (post-scale) geometry from this \
         single reload, final outputs: {:?}",
        comp.outputs(),
    );
}
