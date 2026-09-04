//! Argument encoding for background operations.
//!
//! Background operations are dispatched by name, with their arguments squeezed into a
//! single string. Wherever that string carries free-form user text next to a uuid, the
//! uuid has to come first: a uuid can never contain the separator, so whatever
//! punctuation the user typed survives the round trip.

use uuid::Uuid;

/// Separator between a uuid and the free text that follows it.
const SEP: char = '|';

/// Packs an optional owner uuid ahead of free-form text. The empty prefix means "no
/// owner", which is why the separator is always written.
#[must_use]
pub fn pack_owned(owner: Option<Uuid>, text: &str) -> String {
    match owner {
        Some(uuid) => format!("{uuid}{SEP}{text}"),
        None => format!("{SEP}{text}"),
    }
}

/// Splits what [`pack_owned`] produced back apart. The text comes back verbatim,
/// separators included, because only the first separator is significant.
pub fn unpack_owned(packed: &str) -> Result<(Option<Uuid>, &str), uuid::Error> {
    let (owner, text) = packed.split_once(SEP).unwrap_or(("", packed));
    if owner.is_empty() {
        return Ok((None, text));
    }
    Ok((Some(Uuid::parse_str(owner)?), text))
}
