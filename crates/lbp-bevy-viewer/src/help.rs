//! `--help` text, one entry per printed line.
//!
//! Each flag is its own array entry so branches that add flags touch separate
//! lines; `.gitattributes` marks this file `merge=union`, so two branches that
//! append next to each other merge without a conflict (check the result for a
//! duplicated line when both edited the same entry). Add new flags to
//! `AFTER_CENSUS`, above the final controls line.

use lbp_catalog::census_summary::CENSUS_FLAGS_HELP;

/// Lines printed before the shared census flag help.
pub const BEFORE_CENSUS: &[&str] = &[
    "lbp-bevy-viewer --snapshot FILE --game-dir DIRECTORY [--select-guid HEX]",
    "--play-animation HEX: play an animation on the selected mesh",
    "--animation-frame N: hold that running frame instead of playing (QA captures)",
    "--skinning gpu|cpu: Bevy joint skinning (default) or the CPU reference path",
    "--model-up z|y: single-mesh model up axis (default z: Z-up, front -Y, shown upright facing the camera; y: raw axes)",
    "--browse-query TEXT; --show-unavailable: browser filters",
    "--animation-base: opt-in local animation base-row snapshot (not game time)",
    "--synthetic: authored depth/UV/repeat/clamp scene",
    "--synthetic-groups: authored mixed material groups",
    "--synthetic-affine: authored UV scale/offset/reflection",
    "--qa-capture NEW_PNG_UNDER_LOCAL_ANALYSIS: capture then exit",
    "--qa-hide-ui: draw no panels (clean captures)",
    "--window-16-9: 1280x720 window (the game's 16:9 frame; RPCS3 comparisons)",
    "--preview-lighting: key + ambient preview lighting (not game lighting; levels use their stored settings, approximate)",
    "--lighting-preset N: with a level, light from its stored preset N instead of the main settings (P2l)",
    "--lbp-lighting: preview lighting shaped like the community brdf.cg reconstruction (hemisphere ambient, sun, two rims, squared combine; unfitted stored values, uniform mapping inferred, P2l)",
];

/// Lines printed after the shared census flag help.
pub const AFTER_CENSUS: &[&str] = &[
    "--print-census: print the corpus-count panel text and exit (no window)",
    "--ui-demo: authored UI layer demo (no game data; other options ignored)",
    "--slot-browser: story slot browser over --snapshot/--game-dir (--slot-browser-report prints counts and exits; --slot-browser-slots adds one line per slot)",
    "--text-browser: .trans tables (search by text, tag or key) and font glyph sheets (--text-browser-report prints counts and exits)",
    "--ui-script CLASS: run one of your UI scripts headlessly (stub natives) and draw the frame tree it builds; --ui-script-function NAME (default UpdateUI__Q5Gooey), --ui-script-report prints counts and the outline (--qa-hide-ui draws the tree without the count lines)",
    "--pod-menu CLASS: the Pod main menu as pills on a globe and moon (placeholder spheres; pill positions from the RPCS3 reference), Up/Down/Enter; with a level, the pills over the level view",
    "--pod-camera N: with a level, home view that puts Thing N (the Pod globe) where the RPCS3 frame has it",
    "--pod-shell HEX: with --pod-camera, the Pod shell level (000027b9) as a layer over the view; Space or gamepad Square opens/closes the Puter (camera dolly), Esc or Circle closes; --pod-game-camera: the planets view through the game's own camera (presets, vfov 57.3 deg), World transform and main-menu Earth/Moon placement instead of the globe fit (P2k);",
    "--pod-open: start open; the side panels show the room wall art 0001f3b9 (inferred; --no-pod-wall: plain); --pod-t T: start at opening progress T (0-1, mid-dolly captures); --pod-shell-camera FX,FY,FZ,TX,TY,TZ: closed shell eye from the game camera preset (shell level units); --select-guid HEX (Sackboy 0000043f) stands in front of the DualShock, --play-animation HEX plays on him, --pod-hold-animation HEX while the Puter is open",
    "--puter-screen CLASS: in the 3D view, that UI script's frame tree on a screen quad (arrows/Enter focus); --puter-function NAME, --puter-at X,Y,Z[,WIDTH] (display units, facing +Z; default 1.1,0,0,1.2)",
    "--popit: inventory browser over the disc plans (--popit-report prints counts; add --popit-decode-icons to measure icon decoding)",
    "--level-guid HEX | --level-path MAP_PATH: level scene mode (RENDER_MESH Things at POS)",
    "--level-placement inverse-bind|pos-only: POS Things at POS x inverse bind of bone 0 (default, D120) or POS only (D113)",
    "--shape-bevels: with a level, bevelled shapes (geometry inferred, unverified; default straight)",
    "--runtime-texture MESH:SLOT:TEXTURE (hex GUIDs, slot 0-7, repeatable): with a level (and --pod-shell), draw Solid groups of that mesh whose material lists SLOT as unrendered with TEXTURE (UV0, Repeat, untinted; inferred, game binding unconfirmed)",
    "--level-report: with a level, print scene counts and exit (no window)",
    "--level-focus-thing N: with a level, frame the view on Thing N (reader order)",
    "--level-scripts: with a level, run its scripts at 30 Hz (OnCreate__, then Update__ each tick) and show the counts; --level-scripts-flags N,..: progression flags set on the profile",
    "--play-sackboy: with a level, its shapes as a physics world and Sackboy as a capsule at the first checkpoint (arrows walk and change lanes, Space jumps; camera follows); --play-sackboy-at X,Y,Z spawn; --play-sackboy-input r:40,rj:12,-:30 scripted keys (l r j b f) with the position printed each tick",
    "Left drag orbit; right/middle drag pan; wheel zoom; R reset; F frame.",
];

pub fn print_help() {
    for line in BEFORE_CENSUS {
        println!("{line}");
    }
    println!("{CENSUS_FLAGS_HELP}");
    for line in AFTER_CENSUS {
        println!("{line}");
    }
}
