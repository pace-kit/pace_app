#![allow(non_snake_case)]
mod no_resolver;

use base_resolver::PartialsResolver;
use dioxus::prelude::*;

// If neither feature is enabled,
#[cfg(not(any(feature = "web", feature = "desktop")))]
use no_resolver::Resolver;

// Base case for when both features are enabled
#[cfg(all(feature = "web", feature = "desktop"))]
use no_resolver::Resolver;

// If feature "web" is enabled, use web_resolver
#[cfg(all(feature = "web", not(feature = "desktop")))]
use web_resolver::Resolver;

// If feature "desktop" is enabled, use desktop_resolver
#[cfg(all(feature = "desktop", not(feature = "web")))]
use desktop_resolver::Resolver;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Blog(id: i32) -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go to counter" }
        "Blog post {id}"
    }
}

#[component]
fn Home() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        Link { to: Route::Blog { id: count() }, "Go to blog" }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            {Resolver::HomeSub()}
        }
    }
}
