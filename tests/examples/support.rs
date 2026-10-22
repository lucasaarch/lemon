use lemon::{element::Element, Runtime};

pub fn children(element: &Element) -> &[Element] {
    match element {
        Element::Column(node) | Element::Row(node) | Element::View(node) => &node.children,
        _ => &[],
    }
}

pub fn text(element: &Element) -> Vec<String> {
    if let Element::Text(node) = element {
        return vec![node.content.resolve()];
    }
    children(element).iter().flat_map(text).collect()
}

pub fn click(runtime: &mut Runtime, label: &str) {
    fn find(element: &Element, label: &str) -> Option<std::rc::Rc<dyn Fn()>> {
        if let Element::View(node) = element {
            if text(element) == [label] {
                if let Some(handler) = &node.handlers.on_click {
                    return Some(handler.clone());
                }
            }
        }
        children(element)
            .iter()
            .find_map(|child| find(child, label))
    }
    find(&runtime.root_element().unwrap(), label).expect("clickable control")();
    runtime.flush_effects();
}
