use lemon::{prelude::*, ui::UiState, Viewport};
const SIZE: Viewport = Viewport {
    width: 100.0,
    height: 100.0,
};

#[test]
fn reactive_filters_and_gradients_invalidate_paint_without_relayout() {
    let amount = State::new(0.0);
    let value = amount.clone();
    let radial = State::new(false);
    let fill = radial.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        let colors = (Color::rgb8(255, 0, 0), Color::rgb8(0, 0, 255));
        let gradient: Gradient = if fill.get() {
            RadialGradient::new((0.5, 0.5), 0.5, colors.0, colors.1).into()
        } else {
            LinearGradient::new((0.0, 0.0), (1.0, 1.0), colors.0, colors.1).into()
        };
        View::new()
            .fill()
            .blur(value.get())
            .brightness(1.0 - value.get() * 0.1)
            .gradient(gradient)
            .child(Text::new("Filtered content"))
            .into_element()
    })
    .unwrap();
    ui.update(SIZE, 1.0, None, false).unwrap();
    assert_eq!(ui.scene().effect_layer_count(), 0);
    amount.set(3.0);
    radial.set(true);
    ui.apply_updates().unwrap();
    assert!(!ui.layout_dirty());
    assert!(ui.paint_dirty());
    let root = ui.retained().unwrap().root().unwrap();
    assert_eq!(root.style.blur, 3.0);
    assert_eq!(root.style.brightness, 0.7);
    assert!(matches!(
        root.paint.background_gradient,
        Some(Gradient::Radial(_))
    ));
    ui.update(SIZE, 1.0, None, false).unwrap();
    assert_eq!(ui.scene().effect_layer_count(), 1);
    amount.set(0.0);
    ui.update(SIZE, 1.0, None, false).unwrap();
    assert_eq!(
        ui.scene().effect_layer_count(),
        0,
        "repaint must discard old intermediate layers"
    );
}

#[test]
fn effect_builders_reject_invalid_values_and_stops_are_validated_and_sorted() {
    for invalid in [-1.0, f32::NAN, f32::INFINITY] {
        assert!(std::panic::catch_unwind(|| View::new().blur(invalid)).is_err());
        assert!(std::panic::catch_unwind(|| View::new().brightness(invalid)).is_err());
        assert!(std::panic::catch_unwind(|| GradientStop::new(invalid, Color::default())).is_err());
    }
    assert!(std::panic::catch_unwind(|| RadialGradient::new(
        (0.0, 0.0),
        0.0,
        Color::default(),
        Color::default()
    ))
    .is_err());
    let stops = [
        GradientStop::new(1.0, Color::default()),
        GradientStop::new(0.0, Color::default()),
    ];
    let gradient =
        RadialGradient::new((0.5, 0.5), 0.5, Color::default(), Color::default()).stops(stops);
    assert_eq!(gradient.stops[0].position, 0.0);
    assert!(std::panic::catch_unwind(|| gradient.stops([])).is_err());
}
