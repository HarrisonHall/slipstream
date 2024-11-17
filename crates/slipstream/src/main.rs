#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

use pipes::Pipe;

/// Foo
#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

fn main() {
    let pipe = Pipe::new();
    // dioxus_logger::init(Level::INFO).expect("failed to init logger");
    // info!("starting app");
    // launch(App);
}

fn App() -> Element {
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

// I am a comment!
#[component]
fn Home() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        header { class: "container",
            hgroup {
                h1 { "Slipstream" }
                p { "Feed aggregator and publisher" }
            }
        }
        div { class: "container",
            div { class: "grid",
                button { onclick: move |_| count += 1, "Up high!" }
                button { onclick: move |_| count -= 1, "Down low!" }
            }
        }
    }
}
