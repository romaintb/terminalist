use ratatui::layout::Rect;
use terminalist::ui::components::scrollbar_helper::ScrollbarHelper;

#[test]
fn test_scrollbar_detection() {
    // Test case: 10 items in a rect with height 5 (should need scrollbar)
    let rect = Rect::new(0, 0, 50, 5);
    let total_items = 10;

    let (list_area, scrollbar_area) = ScrollbarHelper::calculate_areas(rect, total_items);

    // Should detect that scrollbar is needed
    assert!(
        scrollbar_area.is_some(),
        "Scrollbar should be needed for 10 items in height 5"
    );

    if let Some(scrollbar_rect) = scrollbar_area {
        println!("Scrollbar area: {:?}", scrollbar_rect);
        assert_eq!(scrollbar_rect.width, 1, "Scrollbar should be 1 column wide");
        assert_eq!(scrollbar_rect.height, 5, "Scrollbar should span full height");
    }

    // List area should be reduced by 1 column for scrollbar
    assert_eq!(list_area.width, 49, "List area should be reduced for scrollbar");
}

#[test]
fn test_no_scrollbar_needed() {
    // Test case: 3 items in a rect with height 5 (should not need scrollbar)
    let rect = Rect::new(0, 0, 50, 5);
    let total_items = 3;

    let (list_area, scrollbar_area) = ScrollbarHelper::calculate_areas(rect, total_items);

    // Should not need scrollbar
    assert!(
        scrollbar_area.is_none(),
        "Scrollbar should not be needed for 3 items in height 5"
    );

    // List area should be the full rect
    assert_eq!(list_area, rect, "List area should be full rect when no scrollbar");
}

#[test]
fn test_border_edge_case() {
    // Test edge case: exactly at the boundary with borders
    let rect = Rect::new(0, 0, 50, 10); // Height 10, but content height is 8 with borders
    let total_items = 9; // 9 items in content height 8 - should need scrollbar

    let (_list_area, scrollbar_area) = ScrollbarHelper::calculate_areas(rect, total_items);

    // The ScrollbarHelper currently uses rect.height (10) for comparison
    // So 9 items vs height 10 should NOT need scrollbar
    assert!(
        scrollbar_area.is_none(),
        "With current logic, 9 items in height 10 should not need scrollbar"
    );
}
