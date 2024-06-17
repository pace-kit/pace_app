use dioxus::prelude::*;

pub struct Resolver;

impl Resolver {
    fn Empty() -> Element {
        rsx! {
            div { "No resolver found" }
        }
    }
}

impl base_resolver::PartialsResolver for Resolver {
    fn HomeSub() -> Element {
        Self::Empty()
    }
}
