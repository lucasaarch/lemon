use super::app;
use lemon::{ui::UiState, Viewport};

#[test]
fn form_stays_centered_as_the_window_resizes() {
    let mut ui = UiState::default();
    ui.mount(app).unwrap();
    for (width, height) in [(900.0, 600.0), (640.0, 480.0)] {
        ui.layout(Viewport { width, height }).unwrap();
        let root = ui.retained().unwrap().root().unwrap();
        let bounds = ui.layout_map().get(root.taffy_id.unwrap()).unwrap();
        let form = ui
            .layout_map()
            .get(root.children[0].taffy_id.unwrap())
            .unwrap();
        assert_eq!((bounds.width, bounds.height), (width, height));
        assert_eq!(form.width, 300.0);
        assert!((form.x - (width - form.width) / 2.0).abs() < 0.01);
        assert!((form.y - (height - form.height) / 2.0).abs() < 0.01);
    }
}
