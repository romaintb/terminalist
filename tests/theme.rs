use ratatui::style::Color;
use terminalist::theme::{format_warnings, Theme, ThemeWarning};

#[test]
fn default_theme_round_trips_through_toml() {
    let theme = Theme::default();
    let toml_str = toml::to_string(&theme).expect("serialize theme");
    let parsed: Theme = toml::from_str(&toml_str).expect("deserialize theme");
    assert_eq!(theme, parsed);
}

#[test]
fn accepts_hex_colors() {
    let toml_str = r##"due_date = "#ff8000""##;
    let theme: Theme = toml::from_str(toml_str).expect("deserialize theme");
    assert_eq!(theme.due_date, Color::Rgb(0xff, 0x80, 0x00));
}

#[test]
fn rejects_invalid_colors() {
    let toml_str = r#"accent = "not-a-color""#;
    let result: Result<Theme, _> = toml::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn format_warnings_returns_none_when_empty() {
    assert_eq!(format_warnings(&[]), None);
}

#[test]
fn format_warnings_summarizes_all_entries() {
    let warnings = vec![
        ThemeWarning {
            field: "danger",
            raw_value: "notacolor".to_string(),
            line: 2,
        },
        ThemeWarning {
            field: "border",
            raw_value: "bogus".to_string(),
            line: 3,
        },
    ];
    let message = format_warnings(&warnings).unwrap();
    assert!(message.contains("2 theme colors"));
    assert!(message.contains("theme.danger (line 2)"));
    assert!(message.contains("theme.border (line 3)"));
}

#[test]
fn theme_warning_display_is_readable() {
    let warning = ThemeWarning {
        field: "danger",
        raw_value: "notacolor".to_string(),
        line: 2,
    };
    assert_eq!(
        warning.to_string(),
        "theme.danger (line 2): 'notacolor' is not a valid color, using default"
    );
}
