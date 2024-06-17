#![allow(non_snake_case)]
use dioxus::prelude::*;

pub trait PartialsResolver {
    fn HomeSub() -> Element;
}
