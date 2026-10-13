//! U1 translation tables (`.trans`, TRANSLATION type 30): LAMS key hash and string lookup.
//!
//! Format rules: CONFIRMED_FROM_COMMUNITY_CODE.
//! https://github.com/ennuo/toolkit/tree/d6a2ef80b5a09999f08f23a803b516096055e5c9
//! lib/cwlib/src/main/java/cwlib/resources/RTranslationTable.java (makeLamsKeyID,
//! byte[] constructor). Independent implementation, MIT knowledge notice in
//! THIRD-PARTY-NOTICES. No resource header, no dependency table, no compression.
//!
//! Binary form (big-endian): i32 count, then `count` rows of {u32 LAMS key, i32 string
//! offset}. Offsets are relative to the end of the row table. A string starts with a U+FEFF
//! mark and runs, in UTF-16 code units, to the next U+FEFF unit or the end of the data.
//! Legacy text form: the data starts with U+FEFF and holds UTF-16 lines alternating tag and
//! value; empty tag lines are skipped; keys are the LAMS hash of the tag.
//!
//! Toolkit keeps the last string for a repeated key (HashMap put). This reader keeps every
//! row in file order, counts repeats, and resolves lookups to the last row like Toolkit.
//! Which files hold which language is not established (docs/ui-research.md PC check 4).
use lbp_core::{ByteReader, Error, ErrorKind, Result};
use serde::Serialize;
use std::collections::HashMap;

/// Byte-order mark that starts every string (binary) and the legacy text form.
pub const STRING_MARK: u16 = 0xFEFF;
/// Size of one binary row: u32 key + i32 offset.
pub const ROW_BYTES: usize = 8;

#[derive(Debug, Clone, Copy)]
pub struct TranslationLimits {
    pub max_entries: usize,
    /// Total UTF-16 code units decoded across all rows (offsets may share strings).
    pub max_total_code_units: usize,
}
impl Default for TranslationLimits {
    fn default() -> Self {
        Self {
            max_entries: 1 << 20,
            max_total_code_units: 64 << 20,
        }
    }
}

/// LAMS key of a translation tag, as Toolkit `makeLamsKeyID` computes it.
///
/// Characters are UTF-16 code units. The first 32 units (space padded) fold base 27 into
/// the low part; units 33..64 fold into the high part, which is seeded with 0xC8509800 for
/// tags of at most 32 units. Units past 64 do not contribute. Result is
/// `low + high * 0xDEADBEEF` modulo 2^32 (Toolkit uses 64-bit wrapping arithmetic; the low
/// 32 bits are the same).
pub fn lams_key(tag: &str) -> u32 {
    let units: Vec<u16> = tag.encode_utf16().collect();
    let fold = |range: std::ops::Range<usize>| {
        range.rev().fold(0u64, |acc, i| {
            let unit = units.get(i).copied().unwrap_or(u16::from(b' '));
            acc.wrapping_mul(27).wrapping_add(u64::from(unit))
        })
    };
    let low = fold(0..32);
    let high = if units.len() > 32 {
        fold(32..64)
    } else {
        0xC850_9800
    };
    low.wrapping_add(high.wrapping_mul(0xDEAD_BEEF)) as u32
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranslationForm {
    Binary,
    LegacyText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationEntry {
    pub key: u32,
    /// Binary form: the raw offset after the row table. `None` for the text form.
    pub string_offset: Option<u32>,
    /// Text form: the tag line. `None` for the binary form.
    pub tag: Option<String>,
    /// `None` only for a text-form tag on the last line with no value line.
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub struct TranslationChecks {
    /// Rows whose key equals an earlier row's key (lookup returns the last).
    pub repeated_keys: usize,
    /// Binary rows whose key is not above the previous key (unsigned).
    pub out_of_order_keys: usize,
    /// Strings that did not start with U+FEFF (decoded big-endian as they are).
    pub strings_without_mark: usize,
    /// Strings that started with a little-endian mark (U+FFFE) and were decoded little-endian.
    pub little_endian_strings: usize,
    /// Strings with unpaired surrogates (replaced with U+FFFD).
    pub invalid_utf16_strings: usize,
    /// Binary rows sharing a string offset with an earlier row.
    pub shared_offsets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationTable {
    pub form: TranslationForm,
    pub entries: Vec<TranslationEntry>,
    /// Absolute start of the string region (binary form; 0 for text form).
    pub strings_start: u64,
    pub data_length: u64,
    pub checks: TranslationChecks,
    lookup: HashMap<u32, usize>,
}

/// Scanner-sized summary: counts only, no strings.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TranslationSummary {
    pub form: TranslationForm,
    pub entry_count: usize,
    pub distinct_keys: usize,
    pub strings_start: u64,
    pub data_length: u64,
    pub checks: TranslationChecks,
}

impl TranslationTable {
    /// The string for a LAMS key; the last row wins for repeated keys (as in Toolkit).
    pub fn get(&self, key: u32) -> Option<&str> {
        self.entries.get(*self.lookup.get(&key)?)?.text.as_deref()
    }
    /// Does the table have a row with this key? True also for a row whose text is
    /// missing, which [`Self::get`] answers with `None`.
    pub fn contains_key(&self, key: u32) -> bool {
        self.lookup.contains_key(&key)
    }
    /// The string for a translation tag (hashed with [`lams_key`]).
    pub fn translate(&self, tag: &str) -> Option<&str> {
        self.get(lams_key(tag))
    }
    pub fn distinct_keys(&self) -> usize {
        self.lookup.len()
    }
    pub fn summary(&self) -> TranslationSummary {
        TranslationSummary {
            form: self.form,
            entry_count: self.entries.len(),
            distinct_keys: self.lookup.len(),
            strings_start: self.strings_start,
            data_length: self.data_length,
            checks: self.checks,
        }
    }
}

/// Parse a whole `.trans` file held in memory.
pub fn parse_translation_table(
    bytes: &[u8],
    limits: TranslationLimits,
) -> Result<TranslationTable> {
    match bytes {
        [0xFE, 0xFF, ..] => parse_text(bytes, limits),
        _ => parse_binary(bytes, limits),
    }
}

fn be_unit(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

fn parse_binary(bytes: &[u8], limits: TranslationLimits) -> Result<TranslationTable> {
    let mut reader = ByteReader::new(bytes);
    let count = reader.be_u32()?;
    if count > i32::MAX as u32 {
        return Err(Error::malformed("translation count is negative"));
    }
    let count = count as usize;
    if count > limits.max_entries {
        return Err(Error::limit(format!(
            "translation count {count} exceeds {}",
            limits.max_entries
        )));
    }
    let strings_start = count
        .checked_mul(ROW_BYTES)
        .and_then(|rows| rows.checked_add(4))
        .ok_or_else(|| Error::malformed("translation row table overflow"))?;
    if strings_start > bytes.len() {
        return Err(Error::malformed("translation row table outside data"));
    }
    let mut entries = Vec::with_capacity(count);
    let mut checks = TranslationChecks::default();
    let mut lookup = HashMap::with_capacity(count);
    let mut seen_offsets = HashMap::with_capacity(count);
    let mut decoded_units = 0usize;
    let mut previous_key = None;
    for index in 0..count {
        let key = reader.be_u32()?;
        let offset = reader.be_u32()?;
        if offset > i32::MAX as u32 {
            return Err(Error::malformed(format!(
                "row {index}: negative string offset"
            )));
        }
        let start = strings_start
            .checked_add(offset as usize)
            .ok_or_else(|| Error::malformed("translation string offset overflow"))?;
        let end = string_end(bytes, start)
            .ok_or_else(|| Error::malformed(format!("row {index}: string outside data")))?;
        decoded_units = decoded_units
            .checked_add((end - start) / 2)
            .filter(|total| *total <= limits.max_total_code_units)
            .ok_or_else(|| Error::limit("translation decoded text budget exceeded"))?;
        let units: Vec<u16> = (start..end)
            .step_by(2)
            .map(|at| be_unit(bytes, at))
            .collect();
        let text = decode_marked(&units, &mut checks);
        if previous_key.is_some_and(|previous| key <= previous) {
            checks.out_of_order_keys += 1;
        }
        previous_key = Some(key);
        if seen_offsets.insert(offset, index).is_some() {
            checks.shared_offsets += 1;
        }
        if lookup.insert(key, index).is_some() {
            checks.repeated_keys += 1;
        }
        entries.push(TranslationEntry {
            key,
            string_offset: Some(offset),
            tag: None,
            text: Some(text),
        });
    }
    Ok(TranslationTable {
        form: TranslationForm::Binary,
        entries,
        strings_start: strings_start as u64,
        data_length: bytes.len() as u64,
        checks,
        lookup,
    })
}

/// End of the string starting at `start`: the next U+FEFF unit after the first unit, or the
/// end of the data. `None` where Toolkit's reader would run past the data (start beyond
/// `len - 2`, or an odd tail with no mark before it).
fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut at = start.checked_add(2).filter(|first| *first <= bytes.len())?;
    while at != bytes.len() {
        if at + 2 > bytes.len() {
            return None;
        }
        if be_unit(bytes, at) == STRING_MARK {
            return Some(at);
        }
        at += 2;
    }
    Some(at)
}

/// Decode one string's code units the way a UTF-16 decoder with mark detection does:
/// a leading U+FEFF is dropped (big-endian), a leading U+FFFE switches to little-endian,
/// anything else is big-endian and kept.
fn decode_marked(units: &[u16], checks: &mut TranslationChecks) -> String {
    let (body, swapped): (&[u16], bool) = match units.first() {
        Some(&STRING_MARK) => (&units[1..], false),
        Some(0xFFFE) => {
            checks.little_endian_strings += 1;
            (&units[1..], true)
        }
        _ => {
            checks.strings_without_mark += 1;
            (units, false)
        }
    };
    let body: Vec<u16> = if swapped {
        body.iter().map(|unit| unit.swap_bytes()).collect()
    } else {
        body.to_vec()
    };
    match String::from_utf16(&body) {
        Ok(text) => text,
        Err(_) => {
            checks.invalid_utf16_strings += 1;
            String::from_utf16_lossy(&body)
        }
    }
}

fn parse_text(bytes: &[u8], limits: TranslationLimits) -> Result<TranslationTable> {
    if !bytes.len().is_multiple_of(2) {
        return Err(Error::malformed(
            "legacy translation text has an odd byte length",
        ));
    }
    if bytes.len() / 2 > limits.max_total_code_units {
        return Err(Error::limit(
            "legacy translation text exceeds the decoded text budget",
        ));
    }
    let units: Vec<u16> = (2..bytes.len())
        .step_by(2)
        .map(|at| be_unit(bytes, at))
        .collect();
    let mut checks = TranslationChecks::default();
    let text = match String::from_utf16(&units) {
        Ok(text) => text,
        Err(_) => {
            checks.invalid_utf16_strings += 1;
            String::from_utf16_lossy(&units)
        }
    };
    let mut lines = java_lines(&text);
    let mut entries = Vec::new();
    let mut lookup = HashMap::new();
    while let Some(tag) = lines.next() {
        if tag.is_empty() {
            continue;
        }
        if entries.len() == limits.max_entries {
            return Err(Error::new(
                ErrorKind::LimitExceeded,
                format!("legacy translation entries exceed {}", limits.max_entries),
            ));
        }
        let key = lams_key(tag);
        if lookup.insert(key, entries.len()).is_some() {
            checks.repeated_keys += 1;
        }
        entries.push(TranslationEntry {
            key,
            string_offset: None,
            tag: Some(tag.to_owned()),
            text: lines.next().map(str::to_owned),
        });
    }
    Ok(TranslationTable {
        form: TranslationForm::LegacyText,
        entries,
        strings_start: 0,
        data_length: bytes.len() as u64,
        checks,
        lookup,
    })
}

/// Lines as Java `BufferedReader.readLine` yields them: split on `\n`, `\r` or `\r\n`;
/// a terminator at the very end does not produce an extra empty line.
fn java_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(text);
    std::iter::from_fn(move || {
        let current = rest?;
        if current.is_empty() {
            rest = None;
            return None;
        }
        match current.find(['\r', '\n']) {
            Some(at) => {
                let skip = if current[at..].starts_with("\r\n") {
                    2
                } else {
                    1
                };
                rest = Some(&current[at + skip..]);
                Some(&current[..at])
            }
            None => {
                rest = None;
                Some(current)
            }
        }
    })
}
