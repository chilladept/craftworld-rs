//! L4 Thing header/part flags and L5 subset: authored synthetic recipes only.
mod common;
use common::{bit, rev, translate, W};
use lbp_level::parts::{ScriptObject, ScriptValue, ShapeColor};
use lbp_level::thing::present_parts;
use lbp_level::{read_payload, LevelReport, Limits, Stop};

fn none() -> bool {
    false
}

const BODY: u32 = 0;
const WORLD: u32 = 2;
const RENDER_MESH: u32 = 3;
const POS: u32 = 4;
const FADER_BIT: u32 = 0x2b;
const SHAPE: u32 = 0x12;
const CREATOR_ANIM_BIT: u32 = 0x29;

fn run(w: &W, head: u32) -> LevelReport {
    read_payload(&w.b, rev(head, w.flags), Limits::default(), &none)
}

#[test]
fn part_flags_creator_anim_shift_and_history() {
    // partsRevision 0x3f: bit 0x29 is CREATOR_ANIM, bit 0x2a is TRANSITION (index 0x29).
    let (p, un) = present_parts(
        0x3f8,
        bit(0) | bit(0x28) | bit(0x29) | bit(0x2a) | bit(0x36),
        0x3f,
    );
    assert_eq!(
        p,
        [
            "BODY",
            "POCKET_ITEM",
            "CREATOR_ANIM",
            "TRANSITION",
            "STREAMING_HINT"
        ]
    );
    assert_eq!(un, 0);
    // partsRevision 0x2f (CONTROLINATOR): no shift; POPPET_POWERUP (history 0x30) absent.
    let (p, un) = present_parts(0x3e2, bit(0x26) | bit(0x27) | bit(0x29), 0x2f);
    assert_eq!(p, ["CONTROLINATOR"]);
    assert_eq!(un, bit(0x27) | bit(0x29));
    // Deprecated indices 0x36..0x3d never appear in the LBP2 band.
    let (p, un) = present_parts(0x3f8, bit(0x3d), 0x2c);
    assert!(p.is_empty());
    assert_eq!(un, bit(0x3d));
}

/// World Thing (BODY, WORLD) whose things array holds A (RENDER_MESH, POS; parent C
/// read inline), C (back-reference) and B (SHAPE, CREATOR_ANIM zero).
fn level(flags: u8) -> W {
    let mut w = W::new(flags);
    w.u32(1).raw(&[0x11; 20]); // cross-play hashes (head > 0x3e6)
    w.u32(1)
        .thing_header(1, 0, 0, 0, 0x3f, bit(BODY) | bit(WORLD));
    w.u32(2).f32(0.).f32(0.).f32(0.).f32(0.).u32(0).u32(0); // BODY
    w.u32(3).u32(3); // WORLD, 3 things
                     // Thing A, parent C inline.
    w.u32(4).u8(0xaa).u32(10);
    w.u32(5).thing_header(30, 1, 0, 0, 0x3f, 0);
    w.u32(0)
        .u32(0)
        .i16(-1)
        .i16(-1)
        .u32(0x77)
        .u8(0)
        .s32(0x3f)
        .u64(bit(RENDER_MESH) | bit(POS));
    w.u32(6).res_guid(0xabc).u32(1).u32(4).res_none();
    w.f32(0.5)
        .f32(1.0)
        .u8(1)
        .f32(0.)
        .f32(0.)
        .u32(0xff00ff00)
        .u8(1)
        .u8(0)
        .u8(3)
        .f32(1.0);
    w.u32(7).u32(0).u32(0).m44(translate(1.0, 2.0, -3.0));
    w.u32(5); // C again: back-reference
              // Thing B: SHAPE then deprecated CREATOR_ANIM.
    w.u32(8)
        .thing_header(20, 1, 0, 0, 0x3f, bit(SHAPE) | bit(CREATOR_ANIM_BIT));
    w.u32(9).u32(4).u8(0);
    for (x, y) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
        w.f32(x).f32(y);
    }
    w.int_vector(&[4])
        .res_guid(0x10)
        .res_none()
        .f32(0.25)
        .f32(1.)
        .u32(0xffffffff);
    w.f32(1.)
        .f32(0.1)
        .m44(translate(0.5, 0.5, 0.))
        .u32(0)
        .u32(0)
        .f32(1.)
        .raw(&[0, 0]);
    w.u32(0).u8(0).i16(3).u32(1).u32(9).u8(0); // contact cache: shared self reference
    w.u8(0).u8(0).raw(&[0; 3]).raw(&[0xff; 2]).u8(0);
    w.u32(0); // CREATOR_ANIM
              // PWorld after the array.
    w.f32(0.).f32(0.).s32(5).s32(6).u32(2).u32(31);
    world_tail_0x3f8(&mut w);
    rlevel_tail_0x3f8(&mut w);
    w
}

/// PWorld fields after thingUIDCounter at head 0x3f8: one EditorSelection (tag 20)
/// naming Thing A by back-reference, everything else empty or zero.
fn world_tail_0x3f8(w: &mut W) {
    w.u32(1).u32(20).s32(3).raw(b"sel").u32(1).u32(4); // selections
    w.u32(0).u32(0).f32(0.).s32(0).u8(1); // backdrop, backdropNew, timer, nightDay, paused
    for _ in 0..5 {
        w.f32(1.);
    }
    w.u32(0)
        .u32(0)
        .u32(0)
        .u8(0)
        .u32(0)
        .u32(0)
        .u32(0)
        .u32(0)
        .u32(0); // rewards, slot, unlocks
    w.s32(4).u32(0).u32(0).u8(0).s32(0); // maxNumPlayers, dissolving, everSpawned, failures
    for _ in 0..11 {
        w.f32(0.);
    }
    w.u8(0).s32(0); // hasLevelLightingBeenSetup, globalSettingsThingUID
    for i in 0..(4 + 9) {
        // camera settings, then the water level, wave and gravity triples
        w.f32(match i {
            4 => 12.5,
            7 => 0.75,
            10 => -2.5,
            _ => 0.,
        });
    }
    w.u32(0).u32(2).u32(40).u32(0).u32(2).u32(1).u32(0); // floatarray, two intarrays
    w.f32(0.3).f32(0.).f32(0.).f32(0.).f32(0.).f32(0.); // tint, tint colour v4, target
    for i in 0..6 {
        w.f32([0.6, 0., 0., 0.9, 0., 0.][i]); // murkiness, bits
    }
    w.u8(1).u8(0).f32(0.).u32(5).s32(0); // drain sounds, wave pos, gameMode, sackbot
    w.u32(0).u32(0).s32(0).s32(0).f32(0.).u8(0); // CutsceneCameraManager
    w.s32(0);
    for _ in 0..10 {
        w.f32(0.); // GlobalAudioSettings
    }
    w.u8(0).u8(0); // backdrop plan descriptors
    w.u8(0).u8(0).s32(0).u8(0).u32(0); // subLevel, scoreLocked, debugTime, checkpoint, cursors
    w.u8(1).u8(4).u8(0).u8(0).u8(0); // min/max players, move, fixInOut, continueMusic
}

/// RLevel fields after the world Thing at head 0x3f8, with one modern InventoryItem.
fn rlevel_tail_0x3f8(w: &mut W) {
    w.u32(0).u32(0).u32(0); // PlayerRecord
    w.u32(1); // tutorialInventory: one item
    w.u8(2).u32(0x99); // plan descriptor (GUID)
    w.u64(0).u32(0).u32(0).u32(0); // dateAdded, SlotID, highlightSound
    w.u32(0).u32(0).u32(0).u32(0).u32(0); // colour, type, subType, title, description
    w.u32(21).u32(1).raw(&[b'x'; 20]); // CreationHistory, one fixed 20-byte name
    w.u8(0); // icon
    w.u32(22).s32(1).raw(&[0, 0x41]).s32(0); // UserCreatedDetails
    w.u32(0).u32(0); // photo data, eyetoy data
    w.i16(0).i16(0).i16(0).u32(0).u8(0).u8(0); // indices, creator, toolType, flags
    w.raw(&[0; 4]).raw(&[0; 4]).raw(&[0; 4]); // fixed BE32 UID, flags, userCategoryIndex
    w.u32(1).u32(5).u32(6).u32(0).raw(&[0; 4]); // tutorialInventoryData: one entry
    w.u8(0).u8(0); // tutorialInventoryActive, copiedFromSomeoneElse
}

#[test]
fn walks_world_things_and_reads_render_parts() {
    for flags in [0, 7] {
        let w = level(flags);
        let r = run(&w, 0x3f8);
        assert_eq!(r.stop, None);
        assert_eq!(r.trailing_bytes, Some(0));
        assert_eq!(r.consumed_bytes, w.b.len());
        assert_eq!(r.cross_play_hashes, Some(1));
        assert_eq!(r.world_things_walked, Some(3));
        let uids: Vec<_> = r.things.iter().map(|t| t.uid.unwrap()).collect();
        assert_eq!(uids, [1, 10, 30, 20]);
        let (world, a, c, b) = (&r.things[0], &r.things[1], &r.things[2], &r.things[3]);
        assert!(world.complete && a.complete && c.complete && b.complete);
        assert_eq!((a.parent, c.parent, b.parent), (Some(2), Some(0), Some(0)));
        assert_eq!(a.plan_guid, Some(0x77));
        assert_eq!(a.parts, ["RENDER_MESH", "POS"]);
        let pos = a.data.pos.as_ref().unwrap();
        assert_eq!(&pos.world[12..15], &[1.0, 2.0, -3.0]);
        let mesh = a.data.render_mesh.as_ref().unwrap();
        assert_eq!(mesh.mesh.unwrap().guid, Some(0xabc));
        assert_eq!(mesh.bone_things, [Some(1)]);
        assert_eq!((mesh.editor_color, mesh.visibility_flags), (0xff00ff00, 3));
        assert_eq!(b.parts, ["SHAPE", "CREATOR_ANIM"]);
        assert_eq!(b.read[1].state, "deprecated_zero");
        let s = b.data.shape.as_ref().unwrap();
        assert_eq!((s.vertex_count, s.requires_z, s.contacts), (4, false, 1));
        assert_eq!(s.vertices[2], [1., 1., 0.]);
        assert_eq!(s.loops, [4]);
        assert_eq!(s.material.unwrap().guid, Some(0x10));
        assert!(matches!(s.color, ShapeColor::Packed(0xffffffff)));
        assert_eq!((s.thickness, s.bevel_size, s.flags), (0.25, 0.1, 3));
        assert_eq!(s.com.unwrap()[12], 0.5);
        let pw = world.data.world.as_ref().unwrap();
        assert_eq!(pw.thing_uid_counter, Some(31));
        // Stage 4: the frame counters, pause flag, water level, game mode and the two
        // global-setting arrays are kept.
        assert_eq!((pw.frame, pw.sim_frame, pw.frame_level_started), (5, 6, 2));
        assert_eq!(
            (pw.is_paused, pw.water_level, pw.game_mode),
            (true, 12.5, 5)
        );
        assert_eq!(pw.global_settings_thing_uids, [40, 0]);
        assert_eq!(pw.global_settings_thing_priority, [1, 0]);
        // D263: the stored current value of the other one-float global settings.
        assert_eq!(
            (pw.water_wave_magnitude, pw.gravity, pw.water_tint),
            (0.75, -2.5, 0.3)
        );
        assert_eq!((pw.water_murkiness, pw.water_bits), (0.6, 0.9));
        assert!(pw.water_drain_sounds_enabled);
        assert_eq!(r.detached_shapes, 0);
        assert_eq!(r.max_depth, 4);
    }
}

#[test]
fn every_truncation_stops_invalid_without_panic() {
    for flags in [0, 7] {
        let w = level(flags);
        for n in 0..w.b.len() {
            let r = read_payload(&w.b[..n], rev(0x3f8, flags), Limits::default(), &none);
            assert!(
                matches!(r.stop, Some(Stop::Invalid { .. })),
                "prefix {n}: {:?}",
                r.stop
            );
            assert!(r.consumed_bytes <= n);
        }
    }
}

#[test]
fn unsupported_part_reports_name_head_and_parts_revision() {
    let mut w = W::new(7);
    w.u32(1)
        .thing_header(1, 0, 0, 0, 0x3c, bit(POS) | bit(FADER_BIT));
    w.u32(2).u32(0).u32(0).m44(translate(0., 0., 0.));
    let r = run(&w, 0x3e5);
    assert_eq!(
        r.stop,
        Some(Stop::UnsupportedPart {
            part: "FADER",
            head: 0x3e5,
            parts_revision: 0x3c,
            offset: w.b.len()
        })
    );
    assert_eq!(r.cross_play_hashes, None);
    assert_eq!(r.things[0].read.len(), 1);
    assert!(r.things[0].data.pos.is_some() && !r.things[0].complete);
}

#[test]
fn null_and_shared_part_references() {
    let mut w = W::new(1);
    w.u32(1)
        .thing_header(1, 0, 0, 0, 0x3f, bit(POS) | bit(SHAPE));
    w.u32(0).u32(0); // POS null, SHAPE null
    w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0); // RLevel tail at 0x3e5, then 2 extra
    w.u8(9).u8(9);
    let r = run(&w, 0x3e5);
    let states: Vec<_> = r.things[0].read.iter().map(|p| p.state).collect();
    assert_eq!(states, ["null", "null"]);
    assert!(r.things[0].complete);
    assert_eq!(r.stop, None);
    assert_eq!(r.trailing_bytes, Some(2));
}

/// D121: one LevelSettings body (head 0x3e5 layout), field values from `base`.
fn level_settings_body(w: &mut W, base: f32) {
    for i in 0..3 {
        w.f32(base + i as f32); // sun position
    }
    w.f32(300000.); // sun position scale
    for v in [1., 0.9, 0.8, 1.] {
        w.f32(v * base); // sun colour
    }
    for v in [0.2, 0.3, 0.4, 1.] {
        w.f32(v); // ambient colour
    }
    w.f32(1.5).f32(1.25); // sun multiplier, exposure
    for v in [0.3, 0.5, 0.7, 1.] {
        w.f32(v); // fog colour
    }
    w.f32(1000.).f32(4000.); // fog near, far
    for _ in 0..8 {
        w.f32(0.5); // rim colours
    }
    // bakedShadowAmount, bakedShadowBlur, bakedAOBias, bakedAOScale, dynamicAOAmount, dofNear
    for v in [0.1, 0.2, 0.25 * base, 0.75, 0.4, 9.] {
        w.f32(v);
    }
    for _ in 0..(1 + 3) {
        w.f32(9.); // dofFar, zEffect (read through)
    }
}

#[test]
fn level_settings_fields_and_presets() {
    const LEVEL_SETTINGS_BIT: u32 = 0xa;
    let mut w = W::new(1);
    w.u32(1)
        .thing_header(1, 0, 0, 0, 0x3f, bit(LEVEL_SETTINGS_BIT));
    w.u32(2);
    level_settings_body(&mut w, 1.);
    w.u32(1); // one preset
    level_settings_body(&mut w, 7.);
    w.s32(3).raw(b"amb").res_none(); // backdropAmbience, backdropMesh
    w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0); // RLevel tail at 0x3e5
    let r = run(&w, 0x3e5);
    assert_eq!(r.stop, None);
    assert_eq!(r.trailing_bytes, Some(0));
    assert_eq!(r.things[0].read[0].state, "parsed");
    let (at, s) = lbp_level::parts::LevelSettings::find(&r.things).unwrap();
    assert_eq!(at, 0);
    assert_eq!(s.sun_position, [1., 2., 3.]);
    assert_eq!(s.sun_position_scale, 300000.);
    assert_eq!(s.sun_color, [1., 0.9, 0.8, 1.]);
    assert_eq!(s.ambient_color, [0.2, 0.3, 0.4, 1.]);
    assert_eq!((s.sun_multiplier, s.exposure), (1.5, 1.25));
    assert_eq!(s.fog_color, [0.3, 0.5, 0.7, 1.]);
    assert_eq!((s.fog_near, s.fog_far), (1000., 4000.));
    assert_eq!((s.rim_color, s.rim_color2), ([0.5; 4], [0.5; 4]));
    assert_eq!(s.presets, 1);
    // P3k: bakedAOBias / bakedAOScale sit after the shadow amount and blur.
    assert_eq!((s.baked_ao_bias, s.baked_ao_scale), (0.25, 0.75));
    // P2l: the preset is kept, in order, with its own values.
    let kept = lbp_level::parts::LevelSettings::find_presets(&r.things);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].sun_position, [7., 8., 9.]);
    assert_eq!(kept[0].sun_color, [1., 0.9, 0.8, 1.].map(|v: f32| v * 7.));
    assert_eq!(kept[0].presets, 0);
    assert_eq!(kept[0].baked_ao_bias, 0.25 * 7.);
    // Every truncation stops without panic and without settings claimed complete.
    for n in 0..w.b.len() {
        let mut t = W::new(1);
        t.raw(&w.b[..n]);
        let r = run(&t, 0x3e5);
        assert!(r.stop.is_some());
    }
}

#[test]
fn shared_part_reference() {
    // The world's groupHead is a new Thing whose POS body is tag 3; the world Thing then
    // names tag 3 again for its own POS, a back-reference to the same part object.
    let mut w = W::new(1);
    w.u32(1).u8(0xaa).u32(1).u32(0);
    w.u32(2).thing_header(2, 0, 0, 0, 0x3f, bit(POS));
    w.u32(3).u32(0).u32(0).m44(translate(5., 0., 0.));
    w.u32(0)
        .i16(0)
        .i16(0)
        .u32(0)
        .u8(0)
        .s32(0x3f)
        .u64(bit(POS))
        .u32(3);
    let r = run(&w, 0x3e5);
    assert_eq!(r.things[1].read[0].state, "parsed");
    assert_eq!(r.things[0].read[0].state, "shared");
    assert_eq!(r.things[0].group_head, Some(1));
    assert!(r.things[0].data.pos.is_none());
}

/// Shape body for tests: 3 vertices, no contacts, head 0x3e5 layout.
type Contact = (u32, Option<fn(&mut W)>);

fn shape_body(w: &mut W, contacts: &[Contact]) {
    w.u32(3).u8(0);
    for (x, y) in [(0., 0.), (1., 0.), (0., 1.)] {
        w.f32(x).f32(y);
    }
    w.int_vector(&[3])
        .res_guid(0x20)
        .res_none()
        .f32(5.)
        .f32(1.)
        .u32(7);
    w.f32(1.)
        .f32(2.)
        .m44(translate(0., 0., 0.))
        .u32(0)
        .u32(0)
        .f32(1.)
        .raw(&[0, 0]);
    w.u32(0).u8(0).i16(0).u32(contacts.len() as u32);
    for (tag, body) in contacts {
        w.u32(*tag);
        if let Some(b) = body {
            b(w);
        }
        w.u8(0);
    }
    w.u8(0).u8(0).raw(&[0; 3]).raw(&[0; 2]).u8(0);
}

/// A PScriptName keeps its name: the stored bytes up to the first NUL.
#[test]
fn script_name_keeps_the_name() {
    let named = |bytes: &[u8]| {
        let mut w = W::new(1);
        w.u32(1).thing_header(1, 0, 0, 0, 0x3f, bit(0xc));
        w.u32(9).s32(bytes.len() as i32).raw(bytes);
        w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0); // RLevel tail
        let r = run(&w, 0x3e5);
        assert_eq!(r.stop, None);
        r.things[0].data.script_name.clone().unwrap()
    };
    // No NUL is stored after a name: every byte belongs to it, in any case and value.
    let n = named(b"Made_Up name\xff");
    assert_eq!(
        (n.stored, n.name.as_slice()),
        (13, &b"Made_Up name\xff"[..])
    );
    // An empty name, and one that is only a NUL.
    let n = named(b"");
    assert_eq!((n.stored, n.name.is_empty()), (0, true));
    let n = named(b"\0");
    assert_eq!((n.stored, n.name.is_empty()), (1, true));
    // A NUL inside ends the name; the bytes after it are read and dropped.
    let n = named(b"up\0down");
    assert_eq!((n.stored, n.name.as_slice()), (7, &b"up"[..]));
    // Reports write the name as text.
    let json = serde_json::to_value(&n).unwrap();
    assert_eq!(json["name"], "up");
}

#[test]
fn contact_shape_is_shared_with_a_later_thing_and_script_name() {
    // Thing 1 has SHAPE tag 2 whose contact introduces shape tag 3 inline (a detached
    // shape). In the second recipe the world's groupHead (Thing 4) does the same and the
    // world Thing then names tag 3 as its own SHAPE, sharing that body.
    let mut w = W::new(1);
    w.u32(1)
        .thing_header(1, 0, 0, 0, 0x3f, bit(0xc) | bit(SHAPE));
    w.u32(9).s32(4).raw(b"ab\0c"); // SCRIPT_NAME: 4 bytes, the name ends at the NUL
    w.u32(2);
    shape_body(&mut w, &[(3, Some(|w: &mut W| shape_body(w, &[])))]);
    w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0); // RLevel tail
    let r = run(&w, 0x3e5);
    assert_eq!(r.stop, None);
    assert_eq!(r.detached_shapes, 1);
    let name = r.things[0].data.script_name.as_ref().unwrap();
    assert_eq!((name.stored, name.name.as_slice()), (4, &b"ab"[..]));
    // Now a second Thing that back-references the contact shape.
    let mut w = W::new(1);
    w.u32(1).u8(0xaa).u32(1).u32(0).u32(4);
    w.thing_header(4, 0, 0, 0, 0x3f, bit(SHAPE)).u32(2);
    shape_body(&mut w, &[(3, Some(|w: &mut W| shape_body(w, &[])))]);
    w.u32(0)
        .i16(0)
        .i16(0)
        .u32(0)
        .u8(0)
        .s32(0x3f)
        .u64(bit(SHAPE))
        .u32(3);
    w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0);
    let r = run(&w, 0x3e5);
    assert_eq!(r.stop, None);
    assert_eq!(r.things[0].read[0].state, "shared");
    let s = r.things[0].data.shape.as_ref().unwrap();
    assert_eq!((s.vertex_count, s.thickness), (3, 5.));
    assert!(matches!(s.color, ShapeColor::Packed(7)));
}

#[test]
fn script_part_keeps_field_names_and_values() {
    // World Thing with a SCRIPT part: script GUID, an inline InstanceLayout of four fields
    // (bool, f32, divergent s32, object = wide string) and their values.
    let field = |w: &mut W, name: &str, modifiers: u16, machine: u8| {
        w.s32(name.len() as i32).raw(name.as_bytes());
        w.raw(&modifiers.to_be_bytes())
            .u8(machine)
            .u8(0)
            .u8(0)
            .u8(0);
        w.u32(0); // instanceOffset
    };
    for flags in [0u8, 1] {
        let mut w = W::new(flags);
        w.u32(1).thing_header(1, 0, 0, 0, 0x3f, bit(0x11));
        w.u32(2).res_guid(0x2d06).u8(1); // PScript tag, script, hasLayout
        w.u32(3).u32(4); // InstanceLayout tag, 4 fields
        field(&mut w, "On", 0, 1);
        field(&mut w, "Speed", 0, 4);
        field(&mut w, "Cache", 1 << 12, 3);
        field(&mut w, "Text", 0, 0xb);
        w.u32(16).u8(0); // instanceSize, reflectDivergent false
        w.u8(1).f32(2.5); // On, Speed (Cache not stored)
        w.u32(14).u32(4).s32(2).raw(&[0, b'H', 0, b'i']); // STRINGW "Hi"
        w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0); // RLevel tail
        let r = run(&w, 0x3e5);
        assert_eq!(r.stop, None, "flags {flags}");
        let p = r.things[0].data.script.as_ref().unwrap();
        assert_eq!(p.script.and_then(|d| d.guid), Some(0x2d06));
        let got: Vec<_> = p
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.machine_type, &f.value))
            .collect();
        assert_eq!(
            got,
            [
                ("On", 1, &ScriptValue::Bool(true)),
                ("Speed", 4, &ScriptValue::Float(2.5)),
                ("Cache", 3, &ScriptValue::NotStored),
                (
                    "Text",
                    0xb,
                    &ScriptValue::Object {
                        kind: 14,
                        id: Some(0),
                        body: Some(Box::new(ScriptObject::Text("Hi".into())))
                    }
                ),
            ]
        );
    }
}

#[test]
fn switch_and_switch_key_keep_the_sensor_fields() {
    // One Thing with a SWITCH (a tag sensor with one output) and a SWITCH_KEY. Head 0x3e5
    // has no playerMode; head 0x3f8 has it.
    for (head, flags) in [(0x3e5, 0u8), (0x3e5, 1), (0x3f8, 0), (0x3f8, 1)] {
        let mut w = W::new(flags);
        if head > 0x3e6 {
            w.u32(1).raw(&[0x11; 20]); // cross-play hashes
        }
        w.u32(1)
            .thing_header(1, 0, 0, 0, 0x3f, bit(0x19) | bit(0x1a));
        w.u32(2).u8(1).f32(30.).f32(5.); // PSwitch tag, inverted, radius, minRadius
        w.s32(3).s32(2).raw(&[0, b'A', 0, b'b']); // colorIndex, name
        w.u8(0).s32(0); // crappyOldLbp1Switch, behaviorOld
        w.u32(1).u32(5).f32(0.5).s32(1).u32(u32::MAX); // one output: tag, signal
        w.u32(0).s32(0); // no targets, userDefinedName
        w.u8(0).s32(4).u32(0); // hideInPlayMode, type, referenceThing
        w.f32(0.).s32(0).u32(u32::MAX); // manualActivation
        w.s32(0).u8(1).f32(45.).s32(0); // holdTime, requireAll, angleRange, includeTouching
        w.s32(2).s32(0).u32(0).u8(0); // bullets required, detected, refresh time, reset
        w.u32(0).u8(0).f32(0.).u32(1).u32(0); // inputList, rigid, timerCount, team, behavior
        for _ in 0..6 {
            w.u32(0); // random*
        }
        w.u8(0).u32(1).u32(0).u8(0).u8(0); // oldJoint, keySensorMode, colour, wires, types
        w.u8(1).u8(0); // detectUnspawnedPlayers, playSwitchAudio
        if head > 0x3ec {
            w.u8(2); // playerMode
        }
        w.u32(6).s32(3).s32(2).raw(&[0, b'A', 0, b'b']).u8(0); // PSwitchKey
        if head > 0x3e6 {
            rlevel_tail_0x3f8(&mut w);
        } else {
            w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0); // RLevel tail
        }
        let r = run(&w, head);
        assert_eq!(r.stop, None, "head {head:x} flags {flags}");
        let s = r.things[0].data.switch.as_ref().unwrap();
        assert_eq!((s.inverted, s.switch_type), (true, 4));
        assert_eq!((s.radius, s.min_radius, s.angle_range), (30., 5., 45.));
        assert_eq!((s.colour_index, s.name.as_str()), (3, "Ab"));
        assert_eq!(
            (s.require_all, s.bullets_required, s.team_filter),
            (true, 2, 1)
        );
        assert_eq!((s.key_sensor_mode, s.detect_unspawned_players), (1, true));
        assert_eq!(s.player_mode, if head > 0x3ec { 2 } else { 0 });
        let o = s.outputs[0].as_ref().unwrap();
        assert_eq!(
            (o.signal.activation, o.signal.ternary, o.signal.player),
            (0.5, 1, -1)
        );
        let k = r.things[0].data.switch_key.as_ref().unwrap();
        assert_eq!((k.colour_index, k.name.as_str()), (3, "Ab"));
    }
}

#[test]
fn script_objects_keep_contents_and_shared_identity() {
    // Fields: a wide string, an f32 array, and an object array holding a back-reference to
    // the string, a null and a nested instance without a layout.
    let field = |w: &mut W, name: &str, machine: u8| {
        w.s32(name.len() as i32).raw(name.as_bytes());
        w.raw(&0u16.to_be_bytes()).u8(machine).u8(0).u8(0).u8(0);
        w.u32(0);
    };
    let mut w = W::new(1);
    w.u32(1).thing_header(1, 0, 0, 0, 0x3f, bit(0x11));
    w.u32(2).res_guid(0x2d06).u8(1);
    w.u32(3).u32(3);
    field(&mut w, "Text", 0xb);
    field(&mut w, "Speeds", 0xb);
    field(&mut w, "Refs", 0xb);
    w.u32(12).u8(0);
    w.u32(16).u32(4).s32(2).raw(b"Hi"); // STRINGA "Hi", tag 4
    w.u32(4).u32(5).u32(2).f32(1.5).f32(-2.); // ARRAY_F32, tag 5
    w.u32(11).u32(6).u32(3); // ARRAY_OBJECT_REF, tag 6, three elements
    w.u32(16).u32(4); // back-reference to tag 4
    w.u32(0); // null
    w.u32(13).u32(7).res_guid(0x2d07).u8(0); // INSTANCE, tag 7, no layout
    w.u32(0).u32(0).u32(0).u32(0).u32(0).u8(0).u8(0);
    let r = run(&w, 0x3e5);
    assert_eq!(r.stop, None);
    let p = r.things[0].data.script.as_ref().unwrap();
    let v: Vec<_> = p.fields.iter().map(|f| &f.value).collect();
    let obj = |v: &ScriptValue| match v {
        ScriptValue::Object { kind, id, body } => (*kind, *id, body.as_deref().cloned()),
        other => panic!("not an object: {other:?}"),
    };
    assert_eq!(
        obj(v[0]),
        (16, Some(0), Some(ScriptObject::Text("Hi".into())))
    );
    assert_eq!(
        obj(v[1]),
        (4, Some(1), Some(ScriptObject::Floats(vec![1.5, -2.])))
    );
    let (kind, id, Some(ScriptObject::Objects(items))) = obj(v[2]) else {
        panic!("object array expected");
    };
    assert_eq!((kind, id, items.len()), (11, Some(2), 3));
    assert_eq!(obj(&items[0]), (16, Some(0), None));
    assert_eq!(obj(&items[1]), (0, None, None));
    let (13, Some(3), Some(ScriptObject::Instance(inst))) = obj(&items[2]) else {
        panic!("instance expected");
    };
    assert_eq!(inst.script.and_then(|d| d.guid), Some(0x2d07));
    assert!(inst.fields.is_empty());
}

#[test]
fn header_violations() {
    // Bad marker.
    let mut w = W::new(1);
    w.u32(1).u8(0xab);
    assert!(matches!(run(&w, 0x3e5).stop, Some(Stop::Invalid { .. })));
    // Deprecated part with nonzero value.
    let mut w = W::new(1);
    w.u32(1)
        .thing_header(1, 0, 0, 0, 0x3f, bit(CREATOR_ANIM_BIT))
        .u32(1);
    assert!(matches!(run(&w, 0x3e5).stop, Some(Stop::Invalid { .. })));
    // Null world.
    let mut w = W::new(1);
    w.u32(0);
    assert!(matches!(run(&w, 0x3e5).stop, Some(Stop::Invalid { .. })));
    // Out of band head and branch.
    assert!(matches!(
        run(&W::new(1), 0x340).stop,
        Some(Stop::Unsupported { .. })
    ));
    assert!(matches!(
        run(&W::new(1), 0x3f9).stop,
        Some(Stop::Unsupported { .. })
    ));
    let mut rv = rev(0x3e5, 1);
    rv.branch_id = 0x4431;
    let r = read_payload(&[], rv, Limits::default(), &none);
    assert!(matches!(r.stop, Some(Stop::Unsupported { .. })));
}

#[test]
fn inline_parent_chain_hits_depth_budget() {
    let mut w = W::new(1);
    for t in 1..=10u32 {
        w.u32(t).u8(0xaa).u32(t);
    }
    let limits = Limits {
        max_depth: 5,
        ..Limits::default()
    };
    let r = read_payload(&w.b, rev(0x3e5, 1), limits, &none);
    assert!(matches!(
        r.stop,
        Some(Stop::Budget {
            what: "reference depth",
            ..
        })
    ));
    assert_eq!(r.max_depth, 5);
    let yes = || true;
    let r = read_payload(&w.b, rev(0x3e5, 1), Limits::default(), &yes);
    assert_eq!(r.stop, Some(Stop::Cancelled));
}

#[test]
fn thing_budget() {
    let mut w = W::new(1);
    w.u32(1).thing_header(1, 2, 0, 0, 0, 0);
    let limits = Limits {
        max_things: 1,
        ..Limits::default()
    };
    let r = read_payload(&w.b, rev(0x3e5, 1), limits, &none);
    assert!(matches!(r.stop, Some(Stop::Budget { what: "things", .. })));
}

#[test]
fn level_lighting_blends_presets_by_world_lighting_and_darkness() {
    use lbp_level::parts::LevelSettings;
    let mk = |sun: f32, exposure: f32, fog: f32, ao_bias: f32| LevelSettings {
        sun_color: [sun, 0., 0., 1.],
        exposure,
        fog_color: [fog, fog, fog, 1.],
        fog_near: 100. * exposure,
        fog_far: 1000.,
        baked_ao_bias: ao_bias,
        baked_ao_scale: 1.,
        ..LevelSettings::default()
    };
    let main = mk(9., 9., 9., 9.);
    let presets = [
        mk(0., 1., 0.1, 0.),
        mk(1., 2., 0.5, 0.5),
        mk(3., 4., 0.9, 0.),
    ];
    let at = |l: f32, d: f32| LevelSettings::at_lighting(&main, &presets, l, d);
    // Spread evenly: 0, 0.5, 1 land on the presets themselves.
    let (s, pos) = at(0., 0.);
    assert_eq!((s.sun_color[0], pos), (0., Some(0.)));
    let (s, pos) = at(0.5, 0.);
    assert_eq!((s.sun_color[0], s.exposure, pos), (1., 2., Some(1.)));
    let (s, pos) = at(1., 0.);
    assert_eq!((s.sun_color[0], s.exposure, pos), (3., 4., Some(2.)));
    // 0.6 of three presets: position 1.2, i.e. 80% preset 1 and 20% preset 2.
    let (s, pos) = at(0.6, 0.);
    assert!((pos.unwrap() - 1.2).abs() < 1e-6);
    assert!((s.sun_color[0] - 1.4).abs() < 1e-6);
    assert!((s.exposure - 2.4).abs() < 1e-6);
    assert!((s.baked_ao_bias - 0.4).abs() < 1e-6);
    assert!((s.fog_near - 240.).abs() < 1e-3);
    // Darkness scales the exposure and the fog colour, nothing else.
    let (d, _) = at(0.6, 0.25);
    assert!((d.exposure - s.exposure * 0.75).abs() < 1e-6);
    assert!((d.fog_color[0] - s.fog_color[0] * 0.75).abs() < 1e-6);
    assert_eq!(d.fog_color[3], 1.);
    assert_eq!(d.sun_color, s.sun_color);
    // Out of range or non-finite lighting clamps; one preset is that preset.
    assert_eq!(at(7., 0.).1, Some(2.));
    assert_eq!(at(f32::NAN, 0.).1, Some(0.));
    assert_eq!(
        LevelSettings::at_lighting(&main, &presets[1..2], 0.9, 0.),
        (presets[1], Some(0.))
    );
    // No presets: the main block, with darkness still applied.
    let (m, pos) = LevelSettings::at_lighting(&main, &[], 0.5, 0.5);
    assert_eq!((m.sun_color, pos), (main.sun_color, None));
    assert_eq!(m.exposure, 4.5);
}

#[test]
fn d332_fog_tint_runs_over_the_level_colour_then_the_palette() {
    use lbp_level::parts::LevelSettings;
    let own = [0.2, 0.4, 0.6, 1.];
    let palette: Vec<[f32; 4]> = (1..=15).map(|i| [i as f32, 0., 0., 1.]).collect();
    let tint = |t: f32| LevelSettings::tinted_fog(own, t, &palette, false);
    // 0 (and below, and not a number) keeps the level's own colour.
    assert_eq!(tint(0.), own);
    assert_eq!(tint(-1.), own);
    assert_eq!(tint(f32::NAN), own);
    // k/8 is palette colour k - 1; between two steps, their blend.
    assert_eq!(tint(1. / 8.), palette[0]);
    assert_eq!(tint(15. / 8.), palette[14]);
    let half = tint(0.5 / 8.);
    assert!((half[0] - 0.6).abs() < 1e-6 && (half[1] - 0.2).abs() < 1e-6);
    assert!((tint(1.5 / 8.)[0] - 1.5).abs() < 1e-5);
    // Past the last colour it blends back to the level's own; 2 and above is that colour.
    assert!((tint(15.5 / 8.)[0] - 7.6).abs() < 1e-5);
    assert_eq!(tint(2.), own);
    assert_eq!(tint(9.), own);
    // The Pod's ring of 7: tint 1 is its own colour again, and the top of the range.
    let pod = &palette[..7];
    assert_eq!(LevelSettings::tinted_fog(own, 7. / 8., pod, true), pod[6]);
    assert_eq!(LevelSettings::tinted_fog(own, 1., pod, true), own);
    assert_eq!(LevelSettings::tinted_fog(own, 1.5, pod, true), own);
    // No palette read: no tint.
    assert_eq!(LevelSettings::tinted_fog(own, 0.5, &[], false), own);
    let s = LevelSettings {
        fog_color: own,
        ..LevelSettings::default()
    };
    assert_eq!(s.with_fog_tint(0.25, &palette, false).fog_color, palette[1]);
}

#[test]
fn d333_fog_pulls_near_to_0_and_far_to_1000() {
    use lbp_level::parts::LevelSettings;
    let s = LevelSettings {
        fog_near: 4000.,
        fog_far: 41000.,
        ..LevelSettings::default()
    };
    let at = |f: f32| {
        let t = s.with_fog(f);
        (t.fog_near, t.fog_far)
    };
    assert_eq!(at(0.), (4000., 41000.));
    assert_eq!(at(f32::NAN), (4000., 41000.));
    assert_eq!(at(1.), (0., 1000.));
    assert_eq!(at(5.), (0., 1000.));
    assert_eq!(at(0.5), (2000., 21000.));
    // Below 0 the other way: near out, far further out (far above 1000).
    assert_eq!(at(-0.5), (6000., 61000.));
    assert_eq!(at(-3.), (8000., 81000.));
}
