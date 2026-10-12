//! U5 InventoryItemDetails and RPlan details tail: authored synthetic recipes only.
mod common;
use common::{rev, W};
use lbp_level::item_details::{read_plan_details_payload, type_names, Placement, Ref};
use lbp_level::serial::{Limits, Stop};

fn none() -> bool {
    false
}

fn wstr(w: &mut W, s: &str) {
    let units: Vec<u16> = s.encode_utf16().collect();
    w.s32(units.len() as i32);
    for u in units {
        w.raw(&u.to_be_bytes());
    }
}

fn desc_guid(w: &mut W, guid: u32) {
    w.u8(2).u32(guid);
}

fn player(w: &mut W, name: &str) {
    let mut n = [0u8; 16];
    n[..name.len()].copy_from_slice(name.as_bytes());
    w.raw(&n).u8(0).raw(&[0; 3]).raw(&[0; 16]);
}

/// RPlan prefix: plan revision word, 3 bytes of thingData.
fn prefix(w: &mut W) {
    w.u32(0x3f8).u32(3).raw(&[1, 2, 3]);
}

/// Modern layout (head > 0x37c) with every reference present once.
fn modern_plan(flags: u8) -> Vec<u8> {
    let mut w = W::new(flags);
    prefix(&mut w);
    w.uleb_or_fixed_s64(-3);
    w.u32(0).u32(7); // levelUnlock DEVELOPER 7
    w.u32(0x55); // highlight sound
    w.u32(0xff00_00ff); // colour
    w.u32(1 << 1 | 1 << 17); // readymade + edit_mode_tool (LBP2)
    w.u32(4); // subType
    w.u32(0x1111).u32(0x2222); // title, description keys
    w.u32(1).u32(1); // CreationHistory tag 1, one creator
    let mut name = [0u8; 20];
    name[..2].copy_from_slice(b"mm");
    w.raw(&name);
    desc_guid(&mut w, 0x4444); // icon
    w.u32(2); // UserCreatedDetails tag 2
    wstr(&mut w, "Box");
    wstr(&mut w, "A box\0");
    w.u32(0); // photo null
    w.u32(3); // EyetoyData tag 3
    w.res_guid(0x10).res_none();
    let mut m = [0f32; 16];
    m[0] = 2.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    w.m44(m);
    for _ in 0..6 {
        w.f32(0.5);
    }
    w.res_none(); // outline (> 0x39f)
    w.i16(5).i16(6).i16(-1); // location/category/primary
    w.u32(4); // creator tag 4
    player(&mut w, "tester");
    w.u8(16).u8(0x80); // toolType, flags
    w.u32(0xaaaa).u32(0xbbbb); // plan location/category keys
    w.b
}

trait S64 {
    fn uleb_or_fixed_s64(&mut self, v: i64) -> &mut Self;
}
impl S64 for W {
    fn uleb_or_fixed_s64(&mut self, v: i64) -> &mut Self {
        if self.flags & 1 != 0 {
            self.uleb(((v << 1) ^ (v >> 63)) as u64)
        } else {
            self.raw(&v.to_be_bytes())
        }
    }
}

#[test]
fn u5_modern_details_values_and_plan_keys() {
    for flags in [0u8, 7] {
        let report = read_plan_details_payload(
            &modern_plan(flags),
            rev(0x3f8, flags),
            Limits::default(),
            &none,
        );
        let p = report
            .result
            .unwrap_or_else(|e| panic!("flags {flags}: {e:?}"));
        assert_eq!(report.trailing_bytes, 0, "flags {flags}: {p:?}");
        assert_eq!(p.thing_data_bytes, 3);
        let d = p.details.unwrap();
        assert_eq!(d.date_added, -3);
        assert_eq!(d.level_unlock.number, 7);
        assert_eq!(d.highlight_sound, Some(0x55));
        assert_eq!(d.title_key, Some(0x1111));
        assert_eq!(d.creation_history, Ref::Value(vec!["mm".to_owned()]));
        assert_eq!(d.icon.unwrap().guid, Some(0x4444));
        let u = d.user_created.value().unwrap();
        assert_eq!((u.name.as_str(), u.description.as_str()), ("Box", "A box"));
        assert_eq!(d.photo, Ref::Null);
        assert_eq!(d.eyetoy.value().unwrap().frame.unwrap().guid, Some(0x10));
        assert_eq!(
            (d.location_index, d.category_index, d.primary_index),
            (5, 6, -1)
        );
        assert_eq!(d.creator, Ref::Value("tester".to_owned()));
        assert_eq!((d.tool_type, d.flags), (16, 0x80));
        assert_eq!(
            type_names(d.type_bits, 0x3f8),
            vec!["readymade", "edit_mode_tool"]
        );
        assert_eq!(
            p.placement,
            Some(Placement::Keys {
                location: 0xaaaa,
                category: 0xbbbb
            })
        );
    }
}

#[test]
fn u5_middle_layout_fixed_fields() {
    // head 0x350: fixed-width words, structs for creator/user details/history.
    let mut w = W::new(0);
    prefix(&mut w);
    for v in [0x55u32, 0, 9, 1, 2, 3, 10, 11, 0] {
        w.raw(&v.to_be_bytes()); // sound, slot, indices, lastUsed, numUses, pad
    }
    w.raw(&(-7i64).to_be_bytes()); // dateAdded
    for v in [100u32, 0xff, 1 << 3, 2, 5] {
        w.raw(&v.to_be_bytes()); // fluff, colour, type (sticker), subType, toolType
    }
    player(&mut w, "mm");
    w.u8(1).u8(0).u8(1).u8(9); // allowEmit, shareable, copyright, flags (> 0x334)
    w.u32(0x77).u32(0x88); // keys (> 0x2ba)
    wstr(&mut w, "");
    wstr(&mut w, "");
    w.u32(1);
    wstr(&mut w, "creator");
    desc_guid(&mut w, 0x99);
    w.u32(0).u32(0); // photo, eyetoy null
    w.u32(0x12).u32(0x34);
    let report = read_plan_details_payload(&w.b, rev(0x350, 0), Limits::default(), &none);
    let p = report.result.unwrap();
    assert_eq!(report.trailing_bytes, 0);
    let d = p.details.unwrap();
    assert_eq!(d.highlight_sound, Some(0x55));
    assert_eq!(d.level_unlock.number, 9);
    assert_eq!(
        (d.location_index, d.category_index, d.primary_index),
        (1, 2, 3)
    );
    assert_eq!(
        (d.last_used, d.num_uses, d.fluff_cost),
        (Some(10), Some(11), Some(100))
    );
    assert_eq!(d.date_added, -7);
    assert_eq!(type_names(d.type_bits, 0x350), vec!["sticker"]);
    assert_eq!(d.tool_type, 5);
    assert_eq!(d.creator, Ref::Value("mm".to_owned()));
    assert_eq!(
        (d.allow_emit, d.shareable, d.copyright),
        (Some(true), Some(false), Some(true))
    );
    assert_eq!(d.flags, 9);
    assert_eq!((d.title_key, d.description_key), (Some(0x77), Some(0x88)));
    assert_eq!(d.creation_history, Ref::Value(vec!["creator".to_owned()]));
    assert_eq!(d.icon.unwrap().guid, Some(0x99));
}

/// NetworkPlayerID below 0x234: every run carries an i32 length.
fn player_prefixed(w: &mut W, name: &str) {
    let mut n = [0u8; 16];
    n[..name.len()].copy_from_slice(name.as_bytes());
    w.u32(16).raw(&n).u8(0).u32(3).raw(&[0; 3]);
    w.u32(8).raw(&[0; 8]).u32(8).raw(&[0; 8]);
}

fn str8(w: &mut W, s: &str) {
    w.s32(s.len() as i32).raw(s.as_bytes());
}

/// Early layout (0x197 <= head < 0x233), U5b.
fn early_plan(head: u32, flags: u8) -> Vec<u8> {
    let mut w = W::new(flags);
    prefix(&mut w);
    str8(&mut w, "BOX");
    for v in [4u32, 5, 6, 0, 1 << 3, 2, 3] {
        w.raw(&v.to_be_bytes()); // location, category, primary, pad, type, subType, tool
    }
    desc_guid(&mut w, 0x99);
    if head > 0x1c0 {
        w.raw(&12u32.to_be_bytes()).raw(&13u32.to_be_bytes()); // numUses, lastUsed
    }
    w.raw(&0x55u32.to_be_bytes()).raw(&0xffu32.to_be_bytes()); // sound, colour
    w.u32(0).u32(0); // eyetoy, photo null
    w.u32(0).raw(&9u32.to_be_bytes()); // levelUnlock: compressed type, fixed number
    w.u8(1); // copyright
    player_prefixed(&mut w, "mm");
    if head > 0x1aa {
        wstr(&mut w, "Box");
        wstr(&mut w, "");
    }
    if head > 0x1b0 {
        w.u32(1);
        wstr(&mut w, "creator");
    }
    if head > 0x204 {
        w.u8(1);
    }
    if head > 0x221 {
        w.raw(&(-7i64).to_be_bytes());
    }
    if head > 0x222 {
        w.u8(0);
    }
    str8(&mut w, "LOC"); // placement tags below 0x2bb
    str8(&mut w, "CAT");
    w.b
}

#[test]
fn u5b_early_layout_fixed_fields_and_tags() {
    for (head, flags) in [(0x19fu32, 0u8), (0x1b0, 0), (0x1fe, 1), (0x232, 3)] {
        let report = read_plan_details_payload(
            &early_plan(head, flags),
            rev(head, flags),
            Limits::default(),
            &none,
        );
        let p = report
            .result
            .unwrap_or_else(|e| panic!("head {head:x}: {e:?}"));
        assert_eq!(report.trailing_bytes, 0, "head {head:x}");
        let d = p.details.unwrap();
        assert_eq!(d.translation_tag.as_deref(), Some("BOX"));
        assert_eq!(
            (d.location_index, d.category_index, d.primary_index),
            (4, 5, 6)
        );
        assert_eq!(type_names(d.type_bits, head), vec!["sticker"]);
        assert_eq!((d.sub_type, d.tool_type), (2, 3));
        assert_eq!(d.icon.unwrap().guid, Some(0x99));
        let uses = (head > 0x1c0).then_some(12);
        assert_eq!(d.num_uses, uses);
        assert_eq!((d.highlight_sound, d.colour), (Some(0x55), 0xff));
        assert_eq!(d.level_unlock.number, 9);
        assert_eq!(d.copyright, Some(true));
        assert_eq!(d.creator, Ref::Value("mm".to_owned()));
        assert_eq!(d.user_created.value().is_some(), head > 0x1aa);
        assert_eq!(d.allow_emit, (head > 0x204).then_some(true));
        assert_eq!(d.date_added, if head > 0x221 { -7 } else { 0 });
        assert_eq!(d.shareable, (head > 0x222).then_some(false));
        assert_eq!(
            p.placement,
            Some(Placement::Tags {
                location: "LOC".into(),
                category: "CAT".into()
            })
        );
    }
    // Every truncation stops with Invalid.
    let good = early_plan(0x1fe, 0);
    for cut in 0..good.len() {
        let r = read_plan_details_payload(&good[..cut], rev(0x1fe, 0), Limits::default(), &none);
        assert!(matches!(r.result, Err(Stop::Invalid { .. })), "cut {cut}");
    }
}

/// Leerdammer branch plan at its head 0x272 (middle layout).
fn leerdammer_plan(keys: bool, flags: u8) -> Vec<u8> {
    let mut w = W::new(flags);
    prefix(&mut w);
    for v in [0x55u32, 0, 9, 1, 2, 3, 10, 11, 0] {
        w.raw(&v.to_be_bytes()); // sound, slot, indices, lastUsed, numUses, pad (> 0x234)
    }
    w.raw(&5i64.to_be_bytes());
    for v in [100u32, 0xff, 1 << 3, 2, 5] {
        w.raw(&v.to_be_bytes());
    }
    player(&mut w, "mm"); // no length prefixes from 0x234
    w.u8(1).u8(0).u8(1); // allowEmit, shareable, copyright (no flags byte below 0x335)
    if keys {
        w.u32(0x77).u32(0x88);
    } else {
        str8(&mut w, "BOX");
    }
    wstr(&mut w, "");
    wstr(&mut w, "");
    w.u32(0); // creation history
    desc_guid(&mut w, 0x99);
    w.u32(0).u32(0); // photo, eyetoy
    if keys {
        w.u32(0x12).u32(0x34);
    } else {
        str8(&mut w, "LOC");
        str8(&mut w, "CAT");
    }
    w.b
}

#[test]
fn u5b_leerdammer_branch_keys_follow_its_revision() {
    for (branch_revision, keys) in [(0x8u16, true), (0x17, true), (0x7, false)] {
        for flags in [0u8, 7] {
            let mut rv = rev(0x272, flags);
            rv.branch_id = 0x4c44;
            rv.branch_revision = branch_revision;
            let report = read_plan_details_payload(
                &leerdammer_plan(keys, flags),
                rv,
                Limits::default(),
                &none,
            );
            let p = report
                .result
                .unwrap_or_else(|e| panic!("branch rev {branch_revision}: {e:?}"));
            assert_eq!(report.trailing_bytes, 0);
            let d = p.details.unwrap();
            assert_eq!(d.date_added, 5);
            assert_eq!(d.creator, Ref::Value("mm".to_owned()));
            assert_eq!(d.icon.unwrap().guid, Some(0x99));
            if keys {
                assert_eq!((d.title_key, d.description_key), (Some(0x77), Some(0x88)));
                assert_eq!(
                    p.placement,
                    Some(Placement::Keys {
                        location: 0x12,
                        category: 0x34
                    })
                );
            } else {
                assert_eq!(d.translation_tag.as_deref(), Some("BOX"));
                assert!(matches!(p.placement, Some(Placement::Tags { .. })));
            }
        }
    }
    // The branch is read only at its own head.
    let mut rv = rev(0x273, 0);
    rv.branch_id = 0x4c44;
    rv.branch_revision = 8;
    let r = read_plan_details_payload(&leerdammer_plan(true, 0), rv, Limits::default(), &none);
    assert!(matches!(r.result, Err(Stop::Unsupported { .. })));
}

#[test]
fn u5_type_bits_follow_the_game_version() {
    // Bit 17 is FLOOD_FILL in LBP1 and EDIT_MODE_TOOL in LBP2; bit 12 PAINT vs SEQUENCER.
    assert_eq!(
        type_names(1 << 17 | 1 << 12, 0x332),
        vec!["paint", "flood_fill"]
    );
    assert_eq!(
        type_names(1 << 17 | 1 << 12, 0x333),
        vec!["sequencer", "edit_mode_tool"]
    );
    assert!(type_names(0, 0x3f8).is_empty());
}

#[test]
fn u5_untrusted_input_stops_explicitly() {
    let lim = Limits::default();
    let good = modern_plan(0);
    for cut in 0..good.len() {
        let r = read_plan_details_payload(&good[..cut], rev(0x3f8, 0), lim, &none);
        assert!(
            matches!(r.result, Err(Stop::Invalid { .. })),
            "cut {cut}: {:?}",
            r.result
        );
    }
    let mut branch = rev(0x3f8, 0);
    branch.branch_id = 0x4431;
    for rv in [branch, rev(0x0021_03f8, 0), rev(0x3f9, 0)] {
        let r = read_plan_details_payload(&good, rv, lim, &none);
        assert!(matches!(r.result, Err(Stop::Unsupported { .. })), "{rv:?}");
    }
    // Modern bytes read with the early layout stop explicitly or misparse, never panic.
    let _ = read_plan_details_payload(&good, rev(0x232, 0), lim, &none);
    // Below PLAN_DETAILS there are no details.
    let mut w = W::new(0);
    prefix(&mut w);
    let r = read_plan_details_payload(&w.b, rev(0x196, 0), lim, &none);
    assert_eq!(r.result.unwrap().details, None);
    // A tag reused across classes is Invalid (same-class reuse would be Ref::Shared).
    let mut w = W::new(0);
    prefix(&mut w);
    let mut body = modern_plan(0)[w.b.len()..].to_vec();
    // Point the creator tag (4) at the UserCreatedDetails tag (2): class mismatch.
    let pos = body.len() - 8 - 2 - 36 - 4;
    body[pos..pos + 4].copy_from_slice(&2u32.to_be_bytes());
    w.raw(&body);
    let r = read_plan_details_payload(&w.b, rev(0x3f8, 0), lim, &none);
    assert!(
        matches!(r.result, Err(Stop::Invalid { .. })),
        "{:?}",
        r.result
    );
}
