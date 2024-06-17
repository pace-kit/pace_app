#![allow(non_snake_case)]
use dioxus::prelude::*;

pub struct Resolver;

impl base_resolver::PartialsResolver for Resolver {
    fn HomeSub() -> Element {
        rsx! {
            div { "Desktop resolver found" }
        }
    }
}
