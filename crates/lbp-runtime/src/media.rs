//! D271 (docs/level-scripts.md, part 22): the video and voice-over requests of the scripts
//! ([`MediaState`]) and the profile's word of LBP2 game progression flags.
//!
//! Nothing here plays a video or a sound. A request is kept as the script made it, so
//! that a later video or audio system can take it from here; the script is told what
//! the game tells it when nothing started.
use crate::LevelWorld;
use lbp_script_vm::Value;
use std::collections::BTreeMap;

/// One `Video.StartVideo__Q5Thinggb` call. No video started for it: the script got
/// [`lbp_script_vm::natives::NO_VIDEO`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoRequest {
    /// The Thing argument (the pod computer's scripts pass null).
    pub thing: Option<u32>,
    /// The GUID of the video.
    pub guid: u32,
    /// The bool argument as passed (what the game does with it was not read).
    pub flag: bool,
    /// `PWorld.Frame` when the call was made.
    pub frame: i32,
}

/// One `Audio.RequestVoiceOver__Q7StringAbb` call. No line started for it: the script
/// got false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceOverRequest {
    /// The name of the line; `None` for a null string.
    pub name: Option<String>,
    /// The two bool arguments as passed. INFERRED: "interrupt the line that plays" and
    /// "play even when the profile has heard it".
    pub flags: [bool; 2],
    /// `PWorld.Frame` when the call was made.
    pub frame: i32,
}

/// The video and voice-over requests of a level world ([`LevelWorld::media`]).
#[derive(Debug, Clone, Default)]
pub struct MediaState {
    /// `Video.StartVideo` calls, oldest first, at most [`MAX_MEDIA_REQUESTS`]; the calls
    /// past that are counted in `videos_dropped`.
    pub videos: Vec<VideoRequest>,
    pub videos_dropped: u64,
    /// `Video.StopVideo__i` calls.
    pub video_stops: u64,
    /// `Audio.RequestVoiceOver` calls, oldest first, at most [`MAX_MEDIA_REQUESTS`]; the
    /// calls past that are counted in `voice_overs_dropped`.
    pub voice_overs: Vec<VoiceOverRequest>,
    pub voice_overs_dropped: u64,
    /// `Audio.StopVoiceOver__` calls.
    pub voice_over_stops: u64,
    /// `Audio.SetMixOverlay__Q7StringAf` and `RemoveMixOverlay__Q7StringAf` calls by the
    /// name of the mix (a null string counts under the empty name).
    pub mix_overlays_set: BTreeMap<String, u64>,
    pub mix_overlays_removed: BTreeMap<String, u64>,
}

/// Most requests [`MediaState::videos`] and [`MediaState::voice_overs`] keep each.
pub const MAX_MEDIA_REQUESTS: usize = 256;

impl MediaState {
    /// All `Video.StartVideo` calls, kept or not.
    pub fn video_requests(&self) -> u64 {
        self.videos.len() as u64 + self.videos_dropped
    }

    /// All `Audio.RequestVoiceOver` calls, kept or not.
    pub fn voice_over_requests(&self) -> u64 {
        self.voice_overs.len() as u64 + self.voice_overs_dropped
    }
}

fn name_of(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::Str(s)) => Some(s.to_string()),
        _ => None,
    }
}

fn guid_of(v: Option<&Value>) -> u32 {
    match v {
        Some(Value::Int(i)) => *i as u32,
        Some(Value::Raw(u) | Value::Ref(u)) => *u,
        _ => 0,
    }
}

impl LevelWorld {
    /// The video and audio calls of [`lbp_script_vm::natives::World::engine_call`]: kept in
    /// [`LevelWorld::media`]. Other names are not for this list.
    pub(crate) fn media_call(&mut self, name: &str, thing: Option<u32>, args: &[Value]) {
        let frame = self.pworld.frame;
        let flag = |i: usize| matches!(args.get(i), Some(Value::Bool(true)));
        let media = &mut self.media;
        match name {
            "StartVideo" if media.videos.len() < MAX_MEDIA_REQUESTS => {
                media.videos.push(VideoRequest {
                    thing,
                    guid: guid_of(args.get(1)),
                    flag: flag(2),
                    frame,
                });
            }
            "StartVideo" => media.videos_dropped += 1,
            "StopVideo" => media.video_stops += 1,
            "RequestVoiceOver" if media.voice_overs.len() < MAX_MEDIA_REQUESTS => {
                media.voice_overs.push(VoiceOverRequest {
                    name: name_of(args.first()),
                    flags: [flag(1), flag(2)],
                    frame,
                });
            }
            "RequestVoiceOver" => media.voice_overs_dropped += 1,
            "StopVoiceOver" => media.voice_over_stops += 1,
            "SetMixOverlay" | "RemoveMixOverlay" => {
                let by_name = if name == "SetMixOverlay" {
                    &mut media.mix_overlays_set
                } else {
                    &mut media.mix_overlays_removed
                };
                *by_name
                    .entry(name_of(args.first()).unwrap_or_default())
                    .or_default() += 1;
            }
            _ => {}
        }
    }
}

/// The value of `--flags` of the `level_scripts` example and of the viewer's
/// `--level-scripts-flags`: bit masks for [`LevelWorld::progression_flags`], separated
/// by commas and ORed together. A mask is decimal, or hexadecimal after `0x`.
pub fn parse_progression_flags(text: &str) -> Result<u32, String> {
    let mut word = 0;
    for mask in text.split(',').map(str::trim) {
        let parsed = match mask.strip_prefix("0x").or_else(|| mask.strip_prefix("0X")) {
            Some(hex) => u32::from_str_radix(hex, 16),
            None => mask.parse::<u32>(),
        };
        word |= parsed.map_err(|e| format!("progression flag mask {mask:?}: {e}"))?;
    }
    Ok(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progression_flag_masks_are_ored() {
        assert_eq!(parse_progression_flags("2"), Ok(2));
        assert_eq!(parse_progression_flags("1, 2,0x10"), Ok(0x13));
        assert_eq!(parse_progression_flags("0X1ffc"), Ok(0x1ffc));
        assert!(parse_progression_flags("").is_err());
        assert!(parse_progression_flags("-1").is_err());
        assert!(parse_progression_flags("two").is_err());
    }
}
