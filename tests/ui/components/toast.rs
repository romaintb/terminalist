//! Layout maths for the corner toast. The rendering itself is eyeballed, but the box
//! has to stay inside its area at every terminal size, so that part gets a check.

use ratatui::layout::Rect;
use terminalist::ui::components::toast;

/// The toast must always land inside its area, borders included, whatever the
/// terminal size or message length.
#[test]
fn rect_stays_inside_its_area() {
    for w in 0..60u16 {
        for h in 0..30u16 {
            let area = Rect::new(3, 2, w, h);
            for len in [0usize, 1, 8, 40, 400, usize::MAX] {
                let Some(r) = toast::rect(area, len) else { continue };
                assert!(r.width >= 3 && r.height >= 3, "degenerate box {r:?}");
                assert!(r.x > area.x && r.y > area.y, "{r:?} not inset in {area:?}");
                assert!(r.right() < area.right(), "{r:?} overflows {area:?}");
                assert!(r.bottom() < area.bottom(), "{r:?} overflows {area:?}");
            }
        }
    }
}

/// A long message grows the box downward rather than being silently clipped.
#[test]
fn rect_grows_for_wrapped_text() {
    let area = Rect::new(0, 0, 40, 20);
    let short = toast::rect(area, 5).unwrap();
    let long = toast::rect(area, 200).unwrap();
    assert_eq!(short.height, 3);
    assert!(long.height > short.height, "{long:?} should wrap onto more lines");
}
