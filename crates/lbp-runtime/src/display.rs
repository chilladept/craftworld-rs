//! D268 (docs/level-scripts.md, part 19): what a display needs to follow the Things that
//! scripts hide, colour or move. The world keeps a list of the Things whose render-mesh
//! visibility, colours, world matrix or (D270) mesh changed since the list was last taken
//! ([`LevelWorld::take_display_dirty`]), each Thing's values as they are now
//! ([`LevelWorld::thing_display`]) and as they were when the level was read
//! ([`LevelWorld::display_at_load`]).
use crate::{transform, LevelWorld};
use std::collections::BTreeMap;

/// The two visibility bits `Thing.HideRenderMesh` clears and `Thing.ShowRenderMesh` sets
/// in a PRenderMesh's visibility byte (FOUND, 01.33: both natives pass the mask 3).
pub const MESH_VISIBILITY_MASK: u8 = 3;

/// The bit of a PRenderMesh's visibility byte that says the mesh is visible in play mode
/// (FOUND, 01.33: the native `PostItNote.IsVisibleInPlayMode__` answers with this bit).
/// The other bit of the mask is then the one for edit mode (INFERRED, also the community
/// reader's name for it).
pub const MESH_VISIBLE_IN_PLAY_MODE: u8 = 1;

/// Meshes below which a visibility change does not go on to the children (FOUND: the two
/// GUIDs the game's visibility call tests the Thing's mesh against).
pub const MESH_VISIBILITY_STOPS: [u32; 2] = [0x27bd, 0x3e6e];

/// World matrices closer than this in every element count as the same place in
/// [`DisplaySummary::moved_far`]: a matrix a script writes back after its own arithmetic
/// differs from the stored one in the last bits. For a Thing far from the origin the
/// threshold grows with its largest element ([`MOVED_FAR_RELATIVE`]), as the rounding
/// does. Thresholds for counting, not game values.
pub const MOVED_FAR: f32 = 0.01;
pub const MOVED_FAR_RELATIVE: f32 = 1.0e-5;

/// Opaque white: the colour a tint has before a script sets one. INFERRED: the game gives
/// a new PRenderMesh one start value for its editor colour and its tint, and the editor
/// colour's default in a level file is white.
pub const WHITE: [f32; 4] = [1.; 4];

/// A colour vector (red, green, blue, alpha) as the packed colour the game keeps
/// (0xAARRGGBB, the order of [`crate::unpack_colour`]). FOUND: the colour setters store a
/// packed 32-bit colour. INFERRED: each channel is kept in 0..1 and rounded to a byte
/// (the game's conversion was not decoded); a channel that is not a number counts as 0.
pub fn pack_colour(c: [f32; 4]) -> u32 {
    let byte = |v: f32| {
        if v.is_nan() {
            0
        } else {
            (v.clamp(0., 1.) * 255.).round() as u32
        }
    };
    byte(c[3]) << 24 | byte(c[0]) << 16 | byte(c[1]) << 8 | byte(c[2])
}

/// What changed on a Thing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisplayChange {
    /// The render mesh's visibility byte.
    pub visibility: bool,
    /// The render mesh's editor colour or tint, or the shape's colour or tint.
    pub colour: bool,
    /// The world matrix.
    pub transform: bool,
    /// The mesh the PRenderMesh shows (`Thing.SetRenderMeshMesh`).
    pub mesh: bool,
}

impl DisplayChange {
    pub const NONE: Self = Self {
        visibility: false,
        colour: false,
        transform: false,
        mesh: false,
    };
    pub const VISIBILITY: Self = Self {
        visibility: true,
        ..Self::NONE
    };
    pub const COLOUR: Self = Self {
        colour: true,
        ..Self::NONE
    };
    pub const TRANSFORM: Self = Self {
        transform: true,
        ..Self::NONE
    };
    pub const MESH: Self = Self {
        mesh: true,
        ..Self::NONE
    };

    pub fn any(&self) -> bool {
        self.visibility || self.colour || self.transform || self.mesh
    }

    pub fn absorb(&mut self, other: Self) {
        self.visibility |= other.visibility;
        self.colour |= other.colour;
        self.transform |= other.transform;
        self.mesh |= other.mesh;
    }
}

/// The values of one Thing that a display draws by.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ThingDisplay {
    /// The PRenderMesh's visibility byte; none for a Thing without a PRenderMesh.
    pub mesh_visibility: Option<u8>,
    /// The PRenderMesh's editor colour: the level file's until a script sets one.
    pub mesh_colour: Option<[f32; 4]>,
    /// The PRenderMesh's editor colour tint; none until a script sets one.
    pub mesh_tint: Option<[f32; 4]>,
    /// The PShape's colour: the level file's until a script sets one.
    pub shape_colour: Option<[f32; 4]>,
    /// The PShape's colour tint; none until a script sets one.
    pub shape_tint: Option<[f32; 4]>,
    /// The PPos world matrix.
    pub world: Option<[f32; 16]>,
    /// GUID of the mesh the PRenderMesh shows: the level file's until a script sets
    /// another; none without a PRenderMesh, or for a mesh without a GUID.
    pub mesh: Option<u32>,
}

impl ThingDisplay {
    /// The Thing has a render mesh that is not visible in play mode
    /// ([`MESH_VISIBLE_IN_PLAY_MODE`] is clear): the state `Thing.HideRenderMesh` leaves,
    /// and the state a level file stores for the meshes only the editor shows. The
    /// level world runs in play mode. INFERRED: the game does not draw such a mesh while
    /// a level is played (the code that draws was not read).
    pub fn mesh_hidden(&self) -> bool {
        self.mesh_visibility
            .is_some_and(|v| v & MESH_VISIBLE_IN_PLAY_MODE == 0)
    }

    /// What is not the same as in `other`. Colours are compared as the packed colours the
    /// game keeps ([`pack_colour`]), a tint that was never set as [`WHITE`]; matrices by
    /// their bits.
    pub fn unlike(&self, other: &Self) -> DisplayChange {
        let colour = |c: Option<[f32; 4]>| c.map(pack_colour);
        let tint = |c: Option<[f32; 4]>| pack_colour(c.unwrap_or(WHITE));
        let bits = |m: Option<[f32; 16]>| m.map(|m| m.map(f32::to_bits));
        DisplayChange {
            visibility: self.mesh_visibility != other.mesh_visibility,
            colour: colour(self.mesh_colour) != colour(other.mesh_colour)
                || tint(self.mesh_tint) != tint(other.mesh_tint)
                || colour(self.shape_colour) != colour(other.shape_colour)
                || tint(self.shape_tint) != tint(other.shape_tint),
            transform: bits(self.world) != bits(other.world),
            mesh: self.mesh != other.mesh,
        }
    }

    /// The matrix that takes the Thing from where `from` has it to where this has it:
    /// world x inverse(from's world). A display that placed something by the old world
    /// matrix (a mesh under its bind pose, a decoration) puts this in front of the
    /// matrix it used. None when either has no world matrix or the old one has no
    /// inverse.
    pub fn moved_from(&self, from: &Self) -> Option<[f32; 16]> {
        let inverse = transform::inverse(&from.world?)?;
        Some(transform::mul(&self.world?, &inverse))
    }

    /// Whether an element of the world matrix is further from `other`'s than the larger
    /// of [`MOVED_FAR`] and [`MOVED_FAR_RELATIVE`] times the largest element of the two
    /// matrices (or only one of the two has a matrix, or an element is not a number).
    pub fn moved_far(&self, other: &Self) -> bool {
        match (self.world, other.world) {
            (Some(a), Some(b)) => {
                let largest = a.iter().chain(&b).fold(0f32, |m, x| m.max(x.abs()));
                let limit = MOVED_FAR.max(MOVED_FAR_RELATIVE * largest);
                a.iter().zip(&b).any(|(a, b)| {
                    let d = (a - b).abs();
                    d.is_nan() || d > limit
                })
            }
            (None, None) => false,
            _ => true,
        }
    }
}

/// The display changes the world keeps ([`LevelWorld::display`]).
#[derive(Debug, Clone, Default)]
pub struct DisplayState {
    /// Things changed since the list was last taken.
    dirty: BTreeMap<u32, DisplayChange>,
    /// The values each changed Thing had when the level was read, kept at its first
    /// change.
    at_load: BTreeMap<u32, ThingDisplay>,
    /// Changes marked since the level was read, by kind (visibility, colour, world
    /// matrix, mesh): a Thing changed twice counts twice.
    pub marks: [u64; 4],
}

/// Things whose display values are not the level file's, at one moment
/// ([`LevelWorld::display_summary`]). Unit: Thing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisplaySummary {
    /// Things with any value unlike the file's.
    pub changed: usize,
    /// Render meshes hidden now and not hidden in the file; and the reverse.
    pub mesh_hidden: usize,
    pub mesh_shown: usize,
    /// Render meshes whose visibility byte is unlike the file's in any bit.
    pub mesh_visibility: usize,
    /// Render meshes whose editor colour, and whose tint, is unlike the file's.
    pub mesh_colour: usize,
    pub mesh_tint: usize,
    /// Shapes whose colour, and whose tint, is unlike the file's.
    pub shape_colour: usize,
    pub shape_tint: usize,
    /// World matrices unlike the file's in any bit; of those, by more than [`MOVED_FAR`];
    /// of those, Things with a render mesh and Things with a shape.
    pub moved: usize,
    pub moved_far: usize,
    pub moved_far_with_mesh: usize,
    pub moved_far_with_shape: usize,
    /// Render meshes that show another mesh than the file's.
    pub mesh_swapped: usize,
}

impl DisplaySummary {
    pub fn absorb(&mut self, o: &Self) {
        self.changed += o.changed;
        self.mesh_hidden += o.mesh_hidden;
        self.mesh_shown += o.mesh_shown;
        self.mesh_visibility += o.mesh_visibility;
        self.mesh_colour += o.mesh_colour;
        self.mesh_tint += o.mesh_tint;
        self.shape_colour += o.shape_colour;
        self.shape_tint += o.shape_tint;
        self.moved += o.moved;
        self.moved_far += o.moved_far;
        self.moved_far_with_mesh += o.moved_far_with_mesh;
        self.moved_far_with_shape += o.moved_far_with_shape;
        self.mesh_swapped += o.mesh_swapped;
    }

    /// The counts by name, in a fixed order (for reports).
    pub fn rows(&self) -> [(&'static str, usize); 13] {
        [
            ("changed", self.changed),
            ("mesh_hidden", self.mesh_hidden),
            ("mesh_shown", self.mesh_shown),
            ("mesh_visibility", self.mesh_visibility),
            ("mesh_colour", self.mesh_colour),
            ("mesh_tint", self.mesh_tint),
            ("shape_colour", self.shape_colour),
            ("shape_tint", self.shape_tint),
            ("moved", self.moved),
            ("moved_far", self.moved_far),
            ("moved_far_with_mesh", self.moved_far_with_mesh),
            ("moved_far_with_shape", self.moved_far_with_shape),
            ("mesh_swapped", self.mesh_swapped),
        ]
    }
}

impl LevelWorld {
    /// The Thing's display values now; none for a Thing the world does not have.
    pub fn thing_display(&self, thing: u32) -> Option<ThingDisplay> {
        let t = self.things.get(thing as usize)?;
        Some(ThingDisplay {
            mesh_visibility: t.render_mesh_visibility,
            mesh_colour: t.render_mesh_colour,
            mesh_tint: t.render_mesh_tint,
            shape_colour: t.shape_colour,
            shape_tint: t.shape_tint,
            world: t.world_matrix,
            mesh: t.mesh_guid,
        })
    }

    /// The Thing's display values when the level was read (its values now when nothing
    /// changed them).
    pub fn display_at_load(&self, thing: u32) -> Option<ThingDisplay> {
        match self.display.at_load.get(&thing) {
            Some(d) => Some(*d),
            None => self.thing_display(thing),
        }
    }

    /// The Things whose display values changed since this was last called, in Thing
    /// order, with what changed. A value written back to what it was before the last
    /// call is still listed: the list says what to look at again, the values say what to
    /// draw.
    pub fn take_display_dirty(&mut self) -> Vec<(u32, DisplayChange)> {
        std::mem::take(&mut self.display.dirty)
            .into_iter()
            .collect()
    }

    /// How many Things are on the list [`Self::take_display_dirty`] would hand out.
    pub fn display_dirty_len(&self) -> usize {
        self.display.dirty.len()
    }

    /// Every Thing whose display values are unlike those it had when the level was read,
    /// in Thing order, with what is unlike. For a display that starts after scripts
    /// already ran.
    pub fn display_changed(&self) -> Vec<(u32, DisplayChange)> {
        let unlike = |(&thing, at_load): (&u32, &ThingDisplay)| {
            let change = self.thing_display(thing)?.unlike(at_load);
            change.any().then_some((thing, change))
        };
        self.display.at_load.iter().filter_map(unlike).collect()
    }

    /// [`Self::display_changed`] as counts.
    pub fn display_summary(&self) -> DisplaySummary {
        let mut s = DisplaySummary::default();
        let packed = |c: Option<[f32; 4]>| c.map(pack_colour);
        let tint = |c: Option<[f32; 4]>| pack_colour(c.unwrap_or(WHITE));
        for (&thing, was) in &self.display.at_load {
            let Some(now) = self.thing_display(thing) else {
                continue;
            };
            let change = now.unlike(was);
            if !change.any() {
                continue;
            }
            s.changed += 1;
            s.mesh_hidden += usize::from(now.mesh_hidden() && !was.mesh_hidden());
            s.mesh_shown += usize::from(!now.mesh_hidden() && was.mesh_hidden());
            s.mesh_visibility += usize::from(change.visibility);
            s.mesh_colour += usize::from(packed(now.mesh_colour) != packed(was.mesh_colour));
            s.mesh_tint += usize::from(tint(now.mesh_tint) != tint(was.mesh_tint));
            s.shape_colour += usize::from(packed(now.shape_colour) != packed(was.shape_colour));
            s.shape_tint += usize::from(tint(now.shape_tint) != tint(was.shape_tint));
            s.moved += usize::from(change.transform);
            s.mesh_swapped += usize::from(change.mesh);
            if now.moved_far(was) {
                s.moved_far += 1;
                s.moved_far_with_mesh += usize::from(now.mesh_visibility.is_some());
                s.moved_far_with_shape += usize::from(now.shape_colour.is_some());
            }
        }
        s
    }

    /// Runs `write` on the Thing and, when a display value is then not what it was, puts
    /// the Thing on the dirty list and keeps the values it had at its first change.
    /// Nothing for a Thing the world does not have.
    pub(crate) fn change_display(
        &mut self,
        thing: u32,
        write: impl FnOnce(&mut crate::ThingState),
    ) {
        let Some(before) = self.thing_display(thing) else {
            return;
        };
        write(&mut self.things[thing as usize]);
        let Some(after) = self.thing_display(thing) else {
            return;
        };
        let change = after.unlike(&before);
        if !change.any() {
            return;
        }
        self.display.at_load.entry(thing).or_insert(before);
        self.display.dirty.entry(thing).or_default().absorb(change);
        let kinds = [
            change.visibility,
            change.colour,
            change.transform,
            change.mesh,
        ];
        for (n, on) in self.display.marks.iter_mut().zip(kinds) {
            *n += u64::from(on);
        }
    }

    /// `Thing.ShowRenderMesh` (`visible`) and `Thing.HideRenderMesh` on a Thing.
    ///
    /// FOUND (01.33, the visibility call both natives reach with the mask 3): nothing for
    /// a Thing without a PRenderMesh. The mask's bits are cleared in the PRenderMesh's
    /// visibility byte and, to show, set again. Then, unless the Thing has a PShape or
    /// its mesh is one of [`MESH_VISIBILITY_STOPS`], the same is done for each child
    /// that has a PRenderMesh and no PShape, and so on down from that child. A child
    /// without a PRenderMesh ends its branch.
    pub(crate) fn show_render_mesh(&mut self, thing: u32, visible: bool) {
        let mut stack = vec![thing];
        // A damaged tree cannot make this run for ever.
        let mut left = self.things.len();
        while let Some(t) = stack.pop().filter(|_| left > 0) {
            left -= 1;
            let Some(state) = self.things.get(t as usize) else {
                continue;
            };
            if state.render_mesh_visibility.is_none() {
                continue;
            }
            let stops = state.parts.contains(&"SHAPE")
                || state
                    .mesh_guid
                    .is_some_and(|g| MESH_VISIBILITY_STOPS.contains(&g));
            let mut child = state.first_child.filter(|_| !stops);
            self.change_display(t, |s| {
                let cleared = s.render_mesh_visibility.unwrap_or(0) & !MESH_VISIBILITY_MASK;
                let bits = if visible { MESH_VISIBILITY_MASK } else { 0 };
                s.render_mesh_visibility = Some(cleared | bits);
            });
            let mut siblings = self.things.len();
            while let Some(c) = child.filter(|_| siblings > 0) {
                siblings -= 1;
                let Some(s) = self.things.get(c as usize) else {
                    break;
                };
                if s.render_mesh_visibility.is_some() && !s.parts.contains(&"SHAPE") {
                    stack.push(c);
                }
                child = s.next_sibling;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_pack_as_the_game_keeps_them() {
        assert_eq!(pack_colour([1., 0., 0., 1.]), 0xffff_0000);
        assert_eq!(pack_colour([0., 0., 1., 0.5]), 0x8000_00ff);
        // Out of range and not a number.
        assert_eq!(pack_colour([2., -1., f32::NAN, 1.]), 0xffff_0000);
        assert_eq!(pack_colour(crate::unpack_colour(0x1234_5678)), 0x1234_5678);
    }

    #[test]
    fn unlike_compares_packed_colours_and_matrix_bits() {
        let a = ThingDisplay {
            mesh_visibility: Some(3),
            mesh_colour: Some([1., 0.5, 0., 1.]),
            world: Some(transform::IDENTITY),
            ..Default::default()
        };
        assert!(!a.unlike(&a).any());
        // A colour that packs to the same bytes is the same colour; a tint of white is
        // the tint never set.
        let mut b = a;
        b.mesh_colour = Some([1., 0.5001, 0., 1.]);
        b.mesh_tint = Some(WHITE);
        assert!(!b.unlike(&a).any());
        b.mesh_tint = Some([1., 0., 0., 1.]);
        assert_eq!(b.unlike(&a), DisplayChange::COLOUR);
        let mut c = a;
        c.mesh_visibility = Some(0);
        assert_eq!(c.unlike(&a), DisplayChange::VISIBILITY);
        assert!(c.mesh_hidden() && !a.mesh_hidden());
        // The play-mode bit decides; the other bit of the mask is the editor's.
        c.mesh_visibility = Some(1);
        assert!(!c.mesh_hidden());
        c.mesh_visibility = Some(2);
        assert!(c.mesh_hidden());
        assert!(!ThingDisplay::default().mesh_hidden());
        let mut d = a;
        let mut m = transform::IDENTITY;
        m[12] = 0.001;
        d.world = Some(m);
        assert_eq!(d.unlike(&a), DisplayChange::TRANSFORM);
        assert!(!d.moved_far(&a));
        // Another mesh, and no mesh.
        let mut e = a;
        e.mesh = Some(7);
        assert_eq!(e.unlike(&a), DisplayChange::MESH);
        assert_eq!(a.unlike(&e), DisplayChange::MESH);
        m[12] = 5.;
        d.world = Some(m);
        assert!(d.moved_far(&a));
        // Far from the origin the rounding is larger, and so is the threshold.
        let (mut far, mut near) = (a, a);
        m[12] = 50_000.;
        far.world = Some(m);
        m[12] = 50_000.25;
        near.world = Some(m);
        assert!(near.unlike(&far).transform && !near.moved_far(&far));
        m[12] = 50_001.;
        near.world = Some(m);
        assert!(near.moved_far(&far));
        // The matrix that takes a placement from the old place to the new one.
        let by = d.moved_from(&a).unwrap();
        assert_eq!(&by[12..15], &[5., 0., 0.]);
        let mut flat = a;
        flat.world = Some([0.; 16]);
        assert!(d.moved_from(&flat).is_none());
        assert!(ThingDisplay::default().moved_from(&a).is_none());
        assert!(ThingDisplay::default().moved_far(&a));
    }

    #[test]
    fn changes_add_up() {
        let mut c = DisplayChange::default();
        assert!(!c.any());
        c.absorb(DisplayChange::COLOUR);
        c.absorb(DisplayChange::TRANSFORM);
        assert!(c.colour && c.transform && !c.visibility && !c.mesh && c.any());
        assert!(DisplayChange::MESH.any() && !DisplayChange::NONE.any());
        assert_eq!(DisplayChange::NONE, DisplayChange::default());
        let mut s = DisplaySummary {
            changed: 1,
            moved: 2,
            ..Default::default()
        };
        s.absorb(&DisplaySummary {
            changed: 2,
            mesh_hidden: 1,
            ..Default::default()
        });
        assert_eq!((s.changed, s.mesh_hidden, s.moved), (3, 1, 2));
        assert_eq!(s.rows()[0], ("changed", 3));
    }
}
