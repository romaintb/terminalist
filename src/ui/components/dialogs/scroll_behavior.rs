use ratatui::widgets::ScrollbarState;

pub fn scroll_up(scroll_offset: &mut usize, scrollbar_state: &mut ScrollbarState) {
    *scroll_offset = scroll_offset.saturating_sub(1);
    *scrollbar_state = scrollbar_state.position(*scroll_offset);
}

pub fn scroll_down(scroll_offset: &mut usize, scrollbar_state: &mut ScrollbarState) {
    *scroll_offset = scroll_offset.saturating_add(1);
    *scrollbar_state = scrollbar_state.position(*scroll_offset);
}

pub fn page_up(scroll_offset: &mut usize, scrollbar_state: &mut ScrollbarState) {
    *scroll_offset = scroll_offset.saturating_sub(10);
    *scrollbar_state = scrollbar_state.position(*scroll_offset);
}

pub fn page_down(scroll_offset: &mut usize, scrollbar_state: &mut ScrollbarState) {
    *scroll_offset = scroll_offset.saturating_add(10);
    *scrollbar_state = scrollbar_state.position(*scroll_offset);
}

pub fn scroll_to_top(scroll_offset: &mut usize, scrollbar_state: &mut ScrollbarState) {
    *scroll_offset = 0;
    *scrollbar_state = scrollbar_state.position(0);
}

pub fn scroll_to_bottom(scroll_offset: &mut usize, scrollbar_state: &mut ScrollbarState) {
    *scroll_offset = usize::MAX;
    *scrollbar_state = scrollbar_state.position(usize::MAX);
}
