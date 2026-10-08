# Skinned animation preview (runtime plan item Rd)

2026-10-08 America/Chicago. D-number to be assigned by the Integrator. Fourth step of
`docs/runtime-research.md` phase R1, after D110 (bones), Rb (`docs/mesh-skin.md`) and Rc
(`docs/animation-keys.md`). This branch is stacked on the Rb and Rc branches (PRs #18,
#19); merge those first.

## What changed

- `lbp_preview::skinned` (renderer-free): `Rig` binds a mesh skeleton to an animation's
  bone list by anim hash; `Rig::skin_matrices(pose)`; `apply_morphs`; `skin_positions`
  (linear-blend skinning); `SkinnedClip` and `load_clip` (read-only, budgeted reads of the
  mesh and the animation); `resolve_animation` (GUID must have confirmed type
  `animation`).
- `lbp-bevy-viewer`: `--play-animation HEX` (and a panel field) plays that animation on
  the selected mesh; `--animation-frame N` holds one running frame for QA captures. The
  status text shows frames, fps, looped, driven/total mesh bones, unmatched animation
  bones and both morph counts. A failed binding leaves the static mesh and says why.
- `lbp-preview` example `skinned_clip_probe` (headless check below).

## Rules and provenance

CONFIRMED_FROM_COMMUNITY_CODE, Toolkit
[d6a2ef80](https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9)
`cwlib/io/exports/MeshExporter.java` (`GLB.FromAnimation`, `createSkeleton`,
`createChildren`) and `cwlib/structs/mesh/Bone.java` (`getLocalTransform`):

| Rule | Detail |
|---|---|
| Hierarchy | The mesh's bones and parents form the joint tree; the animation's own link fields are not used |
| Bind local | root: `skinPose`; child: `inv(parent skinPose) × skinPose` |
| Binding | Animation bone → mesh bone by equal anim hash (`Bone.getNameFromHash`), not by index |
| Pose | Animation rotation/position/scale replace the bone's local TRS (glTF node TRS) |
| Skin matrix | glTF skinning: `world × inverse bind`, inverse bind = `invSkinPose` |

INFERRED (our choices, not format facts):
- Bones the animation does not name keep their bind local transform.
- Morph weight `i` applies to mesh morph `i` (counts can differ; both are shown). Offsets
  are added to bind positions before skinning, as glTF does.
- Weights are normalised by their sum; joints past the bone list and zero-weight vertices
  keep the bind position.
- Integer frames, no interpolation. Clips with `loopStart < frames` wrap into the loop
  range; others restart at frame 0 (the game decides when a one-shot clip ends).
- Display: positions are centred and scaled by the bind-pose bounds every frame, so the
  camera does not follow the motion. Normals stay the bind-pose diagnostic normals
  (the material is unlit).

Not done here: Bevy `SkinnedMesh` GPU skinning, interpolation, blending between clips,
the state→animation map, mirroring (`mirror_bones`), locator events.

## Verification

Cloud (Linux, Rust 1.97.0; CI pins 1.99.0): fmt clean; `cargo test -q -p lbp-preview -p
lbp-bevy-viewer` passes (4 new unit tests: bind pose gives identity skin matrices; hash
binding out of order with an unmatched bone; root rotation moves child-weighted and
blended vertices; scale, unweighted vertices and out-of-range joints; parent range and
cycle rejection); Clippy `-D warnings` clean on both crates.
Not tested here: game data, the window, any visual result.

## PC check (Integrator)

1. Pick the Sackboy body mesh (largest 43-bone set in the D110 skeleton census) and an
   animation whose bone set matches it (`bone_sets_top20` of the Rc animation census).
2. Headless:
   ```powershell
   cargo run -q --release -p lbp-preview --example skinned_clip_probe -- local-analysis/d68-inventory-0133-verified.json "game-data/a1-disc;game-data/a1-update-0133" <MESH> <ANIM>
   ```
   Expected: `matched_bones` 43 (or the mesh's bone count), `nonfinite 0` on every
   frame, bounds within a few times the bind size, nonzero displacement on animated
   frames.
3. Viewer (second monitor): `target/debug/lbp-bevy-viewer.exe --snapshot ... --game-dir
   ... --select-guid <MESH> --play-animation <ANIM>`. Look for: limbs move as one piece
   with the body; no vertices left at the origin; no mirrored or inside-out limbs. Then a
   capture with `--animation-frame <N> --qa-capture local-analysis/<new>.png`.
4. If limbs explode or twist, record which: that tests the column-major and quaternion
   order assumptions (runtime check S4) before any GPU skinning work.

Counts: +0 newly viewable, +0 newly Complete (meshes were already viewable; animation is
not part of Complete).

## Re: GPU skinning (branch claude/gpu-skinning, stacked on Rd)

- The viewer now skins with Bevy `SkinnedMesh` by default; `--skinning cpu` keeps the Rd
  CPU path as the reference. Status text names the path in use.
- Joint entities mirror the mesh hierarchy under a root whose transform is the display
  normalisation N (scale 1/span, translate −centre). Inverse bindposes are
  `invSkinPose × N⁻¹`, so display-space vertices skin to `N × world × invSkinPose × raw`,
  the same result as the CPU path. Each frame writes `Rig::local_matrices(pose)` into
  the joint transforms.
- `lbp_preview::skinned::gpu_skin_attributes` applies the CPU weight rules: weights on
  out-of-range joints or non-positive weights are dropped and the rest normalised; a
  vertex with no usable weight is bound to an extra static joint (index = bone count)
  that leaves it in place. Limit: bone count + 1 ≤ Bevy `MAX_JOINTS` (256), otherwise
  the clip fails with a message pointing to `--skinning cpu`.
- Morphs stay on the CPU: when the mesh and the clip both have morphs, positions are
  rewritten each frame with `SkinnedClip::morphed_bind`, then the GPU skins them.
- Scene meshes get `NoFrustumCulling` (the bind-pose bounds do not cover the motion).

Tests (cloud): `hierarchy_of_locals_reproduces_skin_matrices` and
`gpu_attributes_match_cpu_weight_rules` (lbp-preview);
`gpu_joint_hierarchy_matches_cpu_skin_matrices` (lbp-bevy-viewer: spawns the rig, runs
Bevy transform propagation, compares joint global × inverse bind with
`N × CPU skin matrix × N⁻¹`). Not tested: the GPU shader output on screen.

PC check: same mesh/animation as above, capture `--skinning cpu` and `--skinning gpu`
with the same `--animation-frame`; the two captures should match.

## Next

The state→animation map: no Toolkit serializer exists for ABK/AST/AMP
(`ResourceType` lists them only), and runtime-evidence/sackboy.md marks the anim-per-state
table UNKNOWN. It needs the PC census (do those resource types occur on 01.33?) before
any reader.
