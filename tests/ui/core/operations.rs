//! The encoding that carries an optional owner uuid next to free-form user text.
//! User text is the untrusted half here, so the separator has to survive inside it.

use terminalist::ui::core::operations::{pack_owned, unpack_owned};
use uuid::Uuid;

/// A pipe in the user's text used to eat the uuid and break creation outright.
#[test]
fn text_keeps_its_separators() {
    let owner = Uuid::new_v4();
    for text in [
        "review PR | urgent",
        "|leading",
        "trailing|",
        "a|b|c|d",
        "|||",
        "",
        "plain text",
        "name: with a colon",
    ] {
        let owned = pack_owned(Some(owner), text);
        let (got_owner, got_text) = unpack_owned(&owned).unwrap();
        assert_eq!(got_owner, Some(owner), "owner lost for {text:?}");
        assert_eq!(got_text, text, "text mangled for {text:?}");

        let ownerless = pack_owned(None, text);
        let (no_owner, got_text) = unpack_owned(&ownerless).unwrap();
        assert_eq!(no_owner, None, "phantom owner for {text:?}");
        assert_eq!(got_text, text, "text mangled for {text:?}");
    }
}

/// A garbled uuid has to surface as an error, not as a silently ownerless item.
#[test]
fn a_bad_owner_uuid_is_an_error() {
    assert!(unpack_owned("not-a-uuid|some text").is_err());
}
