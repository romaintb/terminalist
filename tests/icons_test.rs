use terminalist::icons::*;

#[test]
fn test_default_theme() {
    let service = IconService::default();
    assert_eq!(service.theme(), IconTheme::Unicode);
}

#[test]
fn test_emoji_icons() {
    let service = IconService::new(IconTheme::Emoji);
    assert_eq!(service.task_pending(), "🔳");
    assert_eq!(service.task_completed(), "✅");
    assert_eq!(service.task_deleted(), "❌");
}

#[test]
fn test_unicode_icons() {
    let service = IconService::new(IconTheme::Unicode);
    assert_eq!(service.task_pending(), "☐");
    assert_eq!(service.task_completed(), "☒");
    assert_eq!(service.task_deleted(), "✗");
}

#[test]
fn test_ascii_icons() {
    let service = IconService::new(IconTheme::Ascii);
    assert_eq!(service.task_pending(), "[ ]");
    assert_eq!(service.task_completed(), "[X]");
    assert_eq!(service.task_deleted(), "[D]");
}

#[test]
fn test_today_tomorrow_icons() {
    let emoji_service = IconService::new(IconTheme::Emoji);
    assert_eq!(emoji_service.today(), "📅");
    assert_eq!(emoji_service.tomorrow(), "🗓️");

    let unicode_service = IconService::new(IconTheme::Unicode);
    assert_eq!(unicode_service.today(), "◷");
    assert_eq!(unicode_service.tomorrow(), "◶");

    let ascii_service = IconService::new(IconTheme::Ascii);
    assert_eq!(ascii_service.today(), "@");
    assert_eq!(ascii_service.tomorrow(), "+");
}
